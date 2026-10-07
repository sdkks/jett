use ::ratatui::backend::Backend;
use ::std::fs::{self, Metadata};
use ::std::mem::ManuallyDrop;
use ::std::path::PathBuf;
use ::std::sync::mpsc::{Receiver, SyncSender};

use crate::Event;
use crate::config::{Config, ThemeSelection};
use crate::messages::{Instruction, handle_instructions};
use crate::state::dry_run::DryRunPlan;
use crate::state::files::{FileOrFolder, FileTree, Folder};
use crate::state::theme_selector::{ThemeChoice, ThemeSelector};
use crate::state::tiles::Board;
use crate::state::{FileToDelete, UiEffects};
use crate::ui::theme::Theme;
use crate::ui::{CompositionPhase, Display};

#[cfg(test)]
mod composition_tests;
#[cfg(test)]
mod dry_run_tests;
#[cfg(test)]
mod theme_selector_tests;

#[derive(Clone)]
pub enum UiMode {
    Loading,
    Normal,
    ThemeSelector(ThemeSelector),
    ScreenTooSmall,
    DeleteFile(FileToDelete),
    ErrorMessage(String),
    Exiting {
        app_loaded: bool,
    },
    // Keep the warning's deletion context during the dependency migration.
    #[allow(dead_code)]
    WarningMessage(FileToDelete),
}

pub struct App<B>
where
    B: Backend,
{
    pub is_running: bool,
    pub loaded: bool,
    pub ui_mode: UiMode,
    board: Board,
    file_tree: ManuallyDrop<FileTree>,
    display: Display<B>,
    event_sender: SyncSender<Event>,
    ui_effects: UiEffects,
    dry_run_plan: Option<DryRunPlan>,
    theme_choices: Vec<ThemeChoice>,
    theme_name: String,
    config_path: Result<PathBuf, String>,
}

impl<B> App<B>
where
    B: Backend,
{
    pub fn new(
        terminal_backend: B,
        path_in_filesystem: PathBuf,
        event_sender: SyncSender<Event>,
        show_apparent_size: bool,
        disable_delete_confirmation: bool,
        theme: Theme,
    ) -> Self {
        let display = Display::new(terminal_backend, theme);
        let board = Board::new(&Folder::new(&path_in_filesystem));
        let base_folder = Folder::new(&path_in_filesystem);
        let file_tree = ManuallyDrop::new(FileTree::new(
            base_folder,
            path_in_filesystem,
            show_apparent_size,
        ));
        // we use ManuallyDrop here because otherwise the app takes forever to exit
        let mut ui_effects = UiEffects::new();
        ui_effects.delete_confirmation_disabled = disable_delete_confirmation;
        App {
            is_running: true,
            loaded: false,
            board,
            file_tree,
            display,
            ui_mode: UiMode::Loading,
            event_sender,
            ui_effects,
            dry_run_plan: None,
            theme_choices: crate::ui::theme::BUILTIN_SCHEMES
                .iter()
                .map(|name| ThemeChoice {
                    name: (*name).to_owned(),
                    theme: Theme::named(name).expect("built-in theme"),
                })
                .collect(),
            theme_name: "default".to_owned(),
            config_path: crate::config::path().map_err(|error| format!("{error:#}")),
        }
    }
    pub fn configure_dry_run(&mut self, enabled: bool) {
        self.ui_effects.dry_run = enabled;
        self.dry_run_plan = enabled.then(DryRunPlan::default);
    }
    pub fn take_dry_run_plan(&mut self) -> Option<DryRunPlan> {
        self.dry_run_plan.take()
    }
    pub fn configure_theme(&mut self, selection: ThemeSelection) {
        self.theme_choices = selection
            .config
            .scheme_names()
            .into_iter()
            .map(|name| {
                let theme = selection.config.resolve(&name).expect("validated scheme");
                ThemeChoice { name, theme }
            })
            .collect();
        self.theme_name = selection.name;
        self.display.set_theme(selection.theme);
        self.ui_effects.config_notice = selection.notice;
    }
    pub fn open_theme_selector(&mut self) {
        if !matches!(self.ui_mode, UiMode::Normal) {
            return;
        }
        self.ui_mode = UiMode::ThemeSelector(ThemeSelector::new(
            self.theme_choices.clone(),
            self.display.theme(),
            self.theme_name.clone(),
        ));
        self.render();
    }
    pub fn edit_theme_selector(&mut self, edit: impl FnOnce(&mut ThemeSelector)) {
        if let UiMode::ThemeSelector(selector) = &mut self.ui_mode {
            edit(selector);
            if let Some(choice) = selector.selected() {
                self.display.set_theme(choice.theme);
            }
            self.render();
        }
    }
    pub fn cancel_theme_selector(&mut self) {
        if let UiMode::ThemeSelector(selector) = &self.ui_mode {
            self.display.set_theme(selector.original_theme);
            self.theme_name = selector.original_name.clone();
            self.normal_mode();
        }
    }
    pub fn save_theme_selector(&mut self) {
        let UiMode::ThemeSelector(selector) = &self.ui_mode else {
            return;
        };
        let Some(choice) = selector.selected().cloned() else {
            return;
        };
        let result = match &self.config_path {
            Ok(path) => Config::set_scheme(path, &choice.name),
            Err(error) => Err(anyhow::anyhow!("{error}")),
        };
        match result {
            Ok(()) => {
                self.display.set_theme(choice.theme);
                self.theme_name = choice.name;
                self.ui_effects.config_notice = None;
                self.normal_mode();
            }
            Err(error) => {
                self.ui_effects.config_notice = Some(format!("Theme not saved: {error:#}"));
                self.cancel_theme_selector();
            }
        }
    }
    pub fn start(&mut self, receiver: Receiver<Instruction>) {
        handle_instructions(self, receiver);
        self.display.clear();
    }
    pub fn render_and_update_board(&mut self) {
        let current_folder = self.file_tree.get_current_folder();
        self.board.change_files(current_folder);
        self.render();
    }
    pub fn increment_loading_progress_indicator(&mut self) {
        self.ui_effects.increment_loading_progress_indicator();
    }
    pub fn render(&mut self) {
        let full_screen_size = self.display.size();
        if (full_screen_size.width < 50 || full_screen_size.height < 15)
            && !matches!(self.ui_mode, UiMode::ThemeSelector(_))
        {
            self.ui_mode = UiMode::ScreenTooSmall;
        }
        let phase = CompositionPhase::from_scan(self.loaded, self.file_tree.failed_to_read);
        self.display.render(
            &mut self.file_tree,
            &mut self.board,
            &self.ui_mode,
            &self.ui_effects,
            phase,
        );
    }
    pub fn flash_space_freed(&mut self) {
        self.ui_effects.flash_space_freed = true;
    }
    pub fn unflash_space_freed(&mut self) {
        self.ui_effects.flash_space_freed = false;
    }
    pub fn set_path_to_red(&mut self) {
        self.ui_effects.current_path_is_red = true;
    }
    pub fn reset_current_path_color(&mut self) {
        self.ui_effects.current_path_is_red = false;
    }
    pub fn start_ui(&mut self) {
        self.ui_mode = UiMode::Normal;
        self.loaded = true;
        self.render_and_update_board();
    }
    pub fn add_entry_to_base_folder(&mut self, file_metadata: &Metadata, entry_path: PathBuf) {
        self.file_tree.add_entry(file_metadata, &entry_path);
        self.ui_effects.last_read_path = Some(entry_path);
    }
    pub fn reset_ui_mode(&mut self) {
        match self.ui_mode {
            UiMode::Loading | UiMode::Normal | UiMode::ThemeSelector(_) => {}
            _ => {
                self.ui_mode = {
                    if self.loaded {
                        UiMode::Normal
                    } else {
                        UiMode::Loading
                    }
                }
            }
        };
    }
    pub fn show_warning_modal(&mut self) {
        if let Some(file_to_delete) = self.get_file_to_delete() {
            self.ui_mode = UiMode::WarningMessage(file_to_delete);
            self.render();
        }
    }
    pub fn prompt_exit(&mut self) {
        self.ui_mode = UiMode::Exiting {
            app_loaded: self.loaded,
        };
        self.render();
    }
    pub fn exit(&mut self) {
        self.is_running = false;
        // here we do a blocking send rather than a try_send
        // because we want to make sure that if the receiver
        // is active, it received this event so that the app
        // would exit cleanly
        let _ = self.event_sender.send(Event::AppExit);
    }
    pub fn handle_enter(&mut self) {
        if !self.board.has_selected_index() {
            self.board.move_to_largest_folder();
        }
        self.enter_selected();
    }
    pub fn move_selected_right(&mut self) {
        self.board.move_selected_right();
        self.render();
    }
    pub fn move_selected_left(&mut self) {
        self.board.move_selected_left();
        self.render();
    }
    pub fn move_selected_down(&mut self) {
        self.board.move_selected_down();
        self.render();
    }
    pub fn move_selected_up(&mut self) {
        self.board.move_selected_up();
        self.render();
    }
    pub fn enter_selected(&mut self) {
        self.board.record_current_index_and_zoom_level();
        if let Some(tile) = &self.board.currently_selected() {
            let selected_name = &tile.name;
            if let Some(file_or_folder) = self.file_tree.item_in_current_folder(selected_name) {
                match file_or_folder {
                    FileOrFolder::Folder(_) => {
                        self.file_tree.enter_folder(selected_name);
                        self.board.reset_zoom_index();
                        self.board.reset_selected_index();
                        self.render_and_update_board();
                    }
                    FileOrFolder::File(_) => {} // do not enter if currently_selected is a file
                }
            };
        }
    }
    pub fn go_up(&mut self) {
        let succeeded = self.file_tree.leave_folder();
        if let Some((index, zoom_level)) = self.board.pop_previous_index_and_zoom_level() {
            if let Some(index) = index {
                self.board.set_selected_index(&index);
            }
            self.board.set_zoom_index(zoom_level);
        }
        self.render_and_update_board();
        if !succeeded {
            let _ = self.event_sender.try_send(Event::PathError);
        }
    }
    pub fn get_file_to_delete(&self) -> Option<FileToDelete> {
        let currently_selected = self.board.currently_selected()?;
        let mut path_to_file = self.file_tree.current_folder_names.clone();
        path_to_file.push(currently_selected.name.clone());
        let file_to_delete = FileToDelete {
            path_in_filesystem: self.file_tree.path_in_filesystem.clone(),
            path_to_file,
            file_type: currently_selected.file_type,
            num_descendants: currently_selected.descendants,
            size: currently_selected.size,
        };
        Some(file_to_delete)
    }
    pub fn prompt_file_deletion(&mut self) {
        if let Some(file_to_delete) = self.get_file_to_delete() {
            self.ui_mode = UiMode::DeleteFile(file_to_delete.clone());

            if self.ui_effects.delete_confirmation_disabled {
                // Here we just delete the file.
                // As we have set the UI mode above we will get the deletion in progress message box instead of the prompt.
                self.delete_file(&file_to_delete);
            } else {
                // Here we will render which will display the confirmation prompt
                self.render();
            }
        }
    }
    pub fn normal_mode(&mut self) {
        self.ui_mode = UiMode::Normal;
        self.render_and_update_board();
    }
    pub fn delete_file(&mut self, file_to_delete: &FileToDelete) {
        self.ui_effects.deletion_in_progress = true;
        self.render();
        self.ui_effects.deletion_in_progress = false;

        let full_path = file_to_delete.full_path();
        match fs::metadata(&full_path) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                let file_removed = if self.ui_effects.dry_run {
                    Ok(())
                } else if file_type.is_dir() {
                    fs::remove_dir_all(&full_path)
                } else {
                    fs::remove_file(&full_path)
                };
                match file_removed {
                    Ok(_) => {
                        if let Some(plan) = &mut self.dry_run_plan {
                            plan.record(full_path, file_to_delete.size);
                        }
                        self.remove_file_from_ui(file_to_delete);
                        self.ui_mode = UiMode::Normal;
                        self.render_and_update_board();
                        let _ = self.event_sender.try_send(Event::FileDeleted);
                    }
                    Err(msg) => {
                        self.ui_mode = UiMode::ErrorMessage(format!("{}", msg));
                        self.render();
                    }
                };
            }
            Err(msg) => {
                self.ui_mode = UiMode::ErrorMessage(format!("{}", msg));
                self.render();
            }
        }
    }
    pub fn increment_failed_to_read(&mut self) {
        self.file_tree.failed_to_read += 1;
    }
    pub fn zoom_in(&mut self) {
        let current_folder = self.file_tree.get_current_folder();
        self.board.zoom_in(current_folder);
        self.render();
    }
    pub fn zoom_out(&mut self) {
        let current_folder = self.file_tree.get_current_folder();
        self.board.zoom_out(current_folder);
        self.render();
    }
    pub fn reset_zoom(&mut self) {
        let current_folder = self.file_tree.get_current_folder();
        self.board.reset_zoom(current_folder);
        self.render();
    }
    fn remove_file_from_ui(&mut self, file_to_delete: &FileToDelete) {
        self.file_tree.space_freed += file_to_delete.size;
        self.file_tree.delete_file(file_to_delete);
        self.board.reset_selected_index();
    }
}
