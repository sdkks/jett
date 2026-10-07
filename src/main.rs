#[cfg(test)]
mod tests;

mod app;
mod config;
mod input;
mod messages;
mod os;
mod state;
mod ui;

use ::jwalk::WalkDir;
use anyhow::bail;
use clap::{Parser, Subcommand};
use jwalk::Parallelism::{RayonDefaultPool, Serial};
use std::env;
use std::io;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread::park_timeout;
use std::{thread, time};

use ::ratatui::backend::Backend;
use crossterm::event::KeyModifiers;
use crossterm::event::{Event as BackEvent, KeyCode, KeyEvent};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;

use app::{App, UiMode};
use input::TerminalEvents;
use messages::{Event, Instruction, handle_events};
use state::dry_run::{DryRunPlan, FileListDelimiter};
use ui::theme::BUILTIN_SCHEMES;

#[cfg(not(test))]
const SHOULD_SHOW_LOADING_ANIMATION: bool = true;
#[cfg(test)]
const SHOULD_SHOW_LOADING_ANIMATION: bool = false;
#[cfg(not(test))]
const SHOULD_HANDLE_WIN_CHANGE: bool = true;
#[cfg(test)]
const SHOULD_HANDLE_WIN_CHANGE: bool = false;
#[cfg(not(test))]
const SHOULD_SCAN_HD_FILES_IN_MULTIPLE_THREADS: bool = true;
#[cfg(test)]
const SHOULD_SCAN_HD_FILES_IN_MULTIPLE_THREADS: bool = false;

#[derive(Parser, Debug)]
#[command(
    name = "jett",
    version,
    about = "Terminal disk space navigator - jettison the junk",
    args_conflicts_with_subcommands = true
)]
pub struct Opt {
    /// The folder to scan
    folder: Option<PathBuf>,
    #[arg(short, long)]
    /// Show file sizes rather than their block usage on disk
    apparent_size: bool,
    #[arg(short, long)]
    /// Don't ask for confirmation before deleting
    disable_delete_confirmation: bool,
    /// Stay on one filesystem: mounted directories are listed but not scanned (like du -x; Unix only)
    #[arg(short = 'x', long)]
    one_file_system: bool,
    /// Plan confirmed deletions without removing files; show hypothetical freed space
    #[arg(long)]
    dry_run: bool,
    /// Delimiter for the dry-run list (nul is scripting-safe on Unix)
    #[arg(
        long,
        value_enum,
        default_value = "newline",
        requires = "dry_run",
        long_help = "Delimiter for the dry-run list. nul is the only byte that cannot appear inside a Unix filename; newline, tab, and pipe are legal filename characters."
    )]
    file_list_delim: FileListDelimiter,
    /// Color scheme (overrides config; otherwise defaults to the original appearance)
    #[arg(long, value_name = "NAME",
        long_help = format!("Color scheme (overrides config). Built-ins: {}. Custom names defined in config are also accepted. In the navigator, press t to filter and preview themes; Enter saves, Esc cancels.", BUILTIN_SCHEMES.join(", ")))]
    theme: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Read or update the config file without starting the navigator
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Set a config key (rewrites the file without preserving comments)
    Set {
        #[arg(value_parser = ["theme.scheme"])]
        key: String,
        name: String,
    },
    /// Check the config and report specific problems; a missing file is valid
    Validate,
}

fn run_config_command(command: ConfigCommand, path: &std::path::Path) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Set { name, .. } => {
            config::Config::set_scheme(path, &name)?;
            println!("Set theme.scheme = {name:?} in {}", path.display());
        }
        ConfigCommand::Validate => {
            config::Config::load(path)?;
            println!(
                "Config valid: {} (a missing file uses defaults)",
                path.display()
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {err:#}");
        process::exit(2);
    }
}
fn get_stdout() -> io::Result<io::Stdout> {
    Ok(io::stdout())
}

fn try_main() -> Result<(), anyhow::Error> {
    let opts = Opt::parse();
    if let Some(Command::Config { command }) = opts.command {
        return run_config_command(command, &config::path()?);
    }
    let selection = config::select_theme(
        opts.theme.as_deref(),
        config::path().and_then(|path| config::Config::load(&path)),
    )?;

    let plan = match get_stdout() {
        Ok(stdout) => {
            let folder = match opts.folder {
                Some(folder) => folder,
                None => env::current_dir()?,
            };
            if !folder.as_path().is_dir() {
                bail!("Folder '{}' does not exist", folder.to_string_lossy())
            }
            // Keep lexical targets (including symlinks) while making dry-run lists absolute.
            let folder = if opts.dry_run {
                std::path::absolute(folder)?
            } else {
                folder
            };
            enable_raw_mode()?;
            let terminal_backend = CrosstermBackend::new(stdout);
            let terminal_events = TerminalEvents {};
            start_with_theme(
                terminal_backend,
                Box::new(terminal_events),
                folder,
                opts.apparent_size,
                opts.disable_delete_confirmation,
                opts.dry_run,
                opts.one_file_system,
                selection,
            )
        }
        Err(_) => bail!("Failed to get stdout: are you trying to pipe 'jett'?"),
    };
    disable_raw_mode()?;
    if let Some(plan) = plan {
        println!("{}", plan.summary());
        if let Some(path) = plan.finish(opts.file_list_delim)? {
            println!("list: {}", path.display());
        }
    }
    Ok(())
}

// Preserve the default-theme entry point used by the existing snapshot scenarios.
#[cfg(test)]
pub fn start<B>(
    terminal_backend: B,
    terminal_events: Box<dyn Iterator<Item = BackEvent> + Send>,
    path: PathBuf,
    show_apparent_size: bool,
    disable_delete_confirmation: bool,
) where
    B: Backend + Send + 'static,
{
    start_with_theme(
        terminal_backend,
        terminal_events,
        path,
        show_apparent_size,
        disable_delete_confirmation,
        false,
        false,
        config::select_theme(None, Ok(config::Config::default())).expect("default selection"),
    );
}

// Same entry point as start(), but with --one-file-system enabled, for
// exercising the same-device scan path in the snapshot suite.
#[cfg(test)]
pub fn start_one_file_system<B>(
    terminal_backend: B,
    terminal_events: Box<dyn Iterator<Item = BackEvent> + Send>,
    path: PathBuf,
    show_apparent_size: bool,
    disable_delete_confirmation: bool,
) where
    B: Backend + Send + 'static,
{
    start_with_theme(
        terminal_backend,
        terminal_events,
        path,
        show_apparent_size,
        disable_delete_confirmation,
        false,
        true,
        config::select_theme(None, Ok(config::Config::default())).expect("default selection"),
    );
}

#[allow(clippy::too_many_arguments)] // flag plumbing; a config struct would churn every call site
fn start_with_theme<B>(
    terminal_backend: B,
    terminal_events: Box<dyn Iterator<Item = BackEvent> + Send>,
    path: PathBuf,
    show_apparent_size: bool,
    disable_delete_confirmation: bool,
    dry_run: bool,
    one_file_system: bool,
    selection: config::ThemeSelection,
) -> Option<DryRunPlan>
where
    B: Backend + Send + 'static,
{
    let mut active_threads = vec![];

    let (event_sender, event_receiver): (SyncSender<Event>, Receiver<Event>) =
        mpsc::sync_channel(1);
    let (instruction_sender, instruction_receiver): (
        SyncSender<Instruction>,
        Receiver<Instruction>,
    ) = mpsc::sync_channel(100);

    let running = Arc::new(AtomicBool::new(true));
    let loaded = Arc::new(AtomicBool::new(false));

    active_threads.push(
        thread::Builder::new()
            .name("event_executer".to_string())
            .spawn({
                let instruction_sender = instruction_sender.clone();
                || handle_events(event_receiver, instruction_sender)
            })
            .unwrap(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("stdin_handler".to_string())
            .spawn({
                let instruction_sender = instruction_sender.clone();
                let running = running.clone();
                move || {
                    for evt in terminal_events {
                        if let BackEvent::Resize(_x, _y) = evt {
                            if SHOULD_HANDLE_WIN_CHANGE {
                                let _ = instruction_sender.send(Instruction::ResetUiMode);
                                let _ = instruction_sender.send(Instruction::Render);
                            }
                            continue;
                        }

                        if let BackEvent::Key(KeyEvent {
                            code: KeyCode::Char('y'),
                            modifiers: KeyModifiers::NONE,
                            ..
                        })
                        | BackEvent::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            modifiers: KeyModifiers::NONE,
                            ..
                        })
                        | BackEvent::Key(KeyEvent {
                            code: KeyCode::Char('c'),
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        }) = evt
                        {
                            // not ideal, but works in a pinch
                            let _ = instruction_sender.send(Instruction::Keypress(evt));
                            park_timeout(time::Duration::from_millis(100));
                            // if we don't wait, the app won't have time to quit
                            if !running.load(Ordering::Acquire) {
                                // sometimes ctrl-c doesn't shut down the app
                                // (eg. dismissing an error message)
                                // in order not to be aware of those particularities
                                // we check "running"
                                break;
                            }
                        } else if instruction_sender.send(Instruction::Keypress(evt)).is_err() {
                            break;
                        }
                    }
                }
            })
            .unwrap(),
    );

    // Device id of the scan root for --one-file-system (du -x): directories
    // on a different device stay in the listing but their contents are never
    // read. There is no std equivalent on Windows, where the flag is a no-op.
    #[cfg(unix)]
    let root_device: Option<u64> = if one_file_system {
        use ::std::os::unix::fs::MetadataExt;
        std::fs::metadata(&path).ok().map(|metadata| metadata.dev())
    } else {
        None
    };
    #[cfg(not(unix))]
    let _ = one_file_system;

    active_threads.push(
        thread::Builder::new()
            .name("hd_scanner".to_string())
            .spawn({
                let path = path.clone();
                let instruction_sender = instruction_sender.clone();
                let loaded = loaded.clone();
                move || {
                    let walk_dir = WalkDir::new(&path)
                        .parallelism(if SHOULD_SCAN_HD_FILES_IN_MULTIPLE_THREADS {
                            RayonDefaultPool {
                                busy_timeout: time::Duration::from_secs(1),
                            }
                        } else {
                            Serial
                        })
                        .skip_hidden(false)
                        .follow_links(false);
                    // du -x: keep foreign-device directories as (tiny) tiles
                    // but never descend into them.
                    #[cfg(unix)]
                    let walk_dir = match root_device {
                        Some(root_device) => walk_dir.process_read_dir(
                            move |_depth, dir_path, _read_dir_state, children| {
                                use ::std::os::unix::fs::MetadataExt;
                                for child in children.iter_mut() {
                                    let Ok(dir_entry) = child else {
                                        continue;
                                    };
                                    if !dir_entry.file_type().is_dir() {
                                        continue;
                                    }
                                    let child_path = dir_path.join(&dir_entry.file_name);
                                    if let Ok(metadata) = std::fs::metadata(&child_path)
                                        && metadata.dev() != root_device
                                    {
                                        dir_entry.read_children = None;
                                    }
                                }
                            },
                        ),
                        None => walk_dir,
                    };
                    'scanning: for entry in walk_dir.into_iter() {
                        let instruction_sent = match entry {
                            Ok(entry) => match entry.metadata() {
                                Ok(file_metadata) => {
                                    let entry_path = entry.path();
                                    instruction_sender.send(Instruction::AddEntryToBaseFolder((
                                        file_metadata,
                                        entry_path,
                                    )))
                                }
                                Err(_) => {
                                    instruction_sender.send(Instruction::IncrementFailedToRead)
                                }
                            },
                            Err(_) => instruction_sender.send(Instruction::IncrementFailedToRead),
                        };
                        if instruction_sent.is_err() {
                            // if we fail to send an instruction here, this likely means the program has
                            // ended and we need to break this loop as well in order not to hang
                            break 'scanning;
                        };
                    }
                    let _ = instruction_sender.send(Instruction::StartUi);
                    loaded.store(true, Ordering::Release);
                }
            })
            .unwrap(),
    );

    if SHOULD_SHOW_LOADING_ANIMATION {
        active_threads.push(
            thread::Builder::new()
                .name("loading_loop".to_string())
                .spawn({
                    let instruction_sender = instruction_sender.clone();
                    let running = running.clone();
                    move || {
                        while running.load(Ordering::Acquire) && !loaded.load(Ordering::Acquire) {
                            let _ =
                                instruction_sender.send(Instruction::ToggleScanningVisualIndicator);
                            let _ = instruction_sender.send(Instruction::RenderAndUpdateBoard);
                            park_timeout(time::Duration::from_millis(100));
                        }
                    }
                })
                .unwrap(),
        );
    }

    let mut app = App::new(
        terminal_backend,
        path,
        event_sender,
        show_apparent_size,
        disable_delete_confirmation,
        selection.theme,
    );
    app.configure_dry_run(dry_run);
    app.configure_theme(selection);
    app.start(instruction_receiver);
    running.store(false, Ordering::Release);

    for thread_handler in active_threads {
        thread_handler.join().unwrap();
    }
    app.take_dry_run_plan()
}
