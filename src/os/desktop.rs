use ::std::io::Write;
use ::std::path::Path;
use ::std::process::{Command, Stdio};

// The command tables are plain data, separate from spawning, so tests can
// verify the platform mapping without executing anything: jett is only ever
// run on real user machines, never in the test container.
//
// Windows commands (explorer, clip) are implemented but untested on real
// Windows, like the rest of this module's Windows support. Note that
// explorer returns exit code 1 even on success; we never wait on the spawned
// process, so that quirk is irrelevant here.
pub(crate) fn folder_opener() -> (&'static str, Vec<&'static str>) {
    if cfg!(target_os = "macos") {
        ("open", vec![])
    } else if cfg!(target_os = "windows") {
        ("explorer", vec![])
    } else {
        ("xdg-open", vec![])
    }
}

pub(crate) fn clipboard_candidates() -> Vec<(&'static str, Vec<&'static str>)> {
    if cfg!(target_os = "macos") {
        vec![("pbcopy", vec![])]
    } else if cfg!(target_os = "windows") {
        vec![("clip", vec![])]
    } else {
        vec![
            // Wayland first, then the X11 tools most distros ship.
            ("wl-copy", vec![]),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ]
    }
}

fn not_found_message(program: &str) -> String {
    format!("{program} not found; install it or extend PATH")
}

pub(crate) fn clipboard_missing_message() -> String {
    let names: Vec<&str> = clipboard_candidates()
        .iter()
        .map(|(program, _)| *program)
        .collect();
    format!(
        "no clipboard tool found (tried {}); install one to copy paths",
        names.join(", ")
    )
}

// Spawn the system file manager on the folder containing the selected item.
// The process is intentionally not waited on: openers detach themselves.
pub(crate) fn open_folder(folder: &Path) -> Result<(), String> {
    let (program, args) = folder_opener();
    let spawn = Command::new(program).args(&args).arg(folder).spawn();
    match spawn {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => Err(not_found_message(program)),
        Err(err) => Err(format!("{program} failed to start: {err}")),
    }
}

// Copy text to the system clipboard by piping it to the first available
// clipboard tool. Like open_folder, children are never waited on: X11
// clipboard owners must keep running to serve the selection.
pub(crate) fn copy_to_clipboard(text: &str) -> Result<(), String> {
    for (program, args) in clipboard_candidates() {
        let spawn = Command::new(program)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawn {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.as_mut() {
                    // Ignore broken pipes: some tools exit after reading.
                    let _ = stdin.write_all(text.as_bytes());
                }
                return Ok(());
            }
            // Tool not installed: try the next candidate.
            Err(err) if err.kind() == ::std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(format!("{program} failed to start: {err}")),
        }
    }
    Err(clipboard_missing_message())
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn linux_opens_folders_with_xdg_open() {
        assert_eq!(folder_opener(), ("xdg-open", vec![]));
    }

    #[test]
    fn linux_tries_wayland_then_x11_clipboard_tools() {
        let names: Vec<&str> = clipboard_candidates()
            .iter()
            .map(|(program, _)| *program)
            .collect();
        assert_eq!(names, vec!["wl-copy", "xclip", "xsel"]);
        assert_eq!(clipboard_candidates()[1].1, vec!["-selection", "clipboard"]);
    }

    #[test]
    fn missing_clipboard_message_lists_every_candidate() {
        assert_eq!(
            clipboard_missing_message(),
            "no clipboard tool found (tried wl-copy, xclip, xsel); install one to copy paths"
        );
    }
}
