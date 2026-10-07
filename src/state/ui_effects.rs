use ::std::path::PathBuf;

pub struct UiEffects {
    pub dry_run: bool,
    pub delete_confirmation_disabled: bool,
    pub flash_space_freed: bool,
    pub current_path_is_red: bool,
    pub deletion_in_progress: bool,
    pub loading_progress_indicator: u64,
    pub last_read_path: Option<PathBuf>,
    pub config_notice: Option<String>,
}

impl UiEffects {
    pub fn new() -> Self {
        Self {
            dry_run: false,
            delete_confirmation_disabled: false,
            flash_space_freed: false,
            current_path_is_red: false,
            deletion_in_progress: false,
            loading_progress_indicator: 0,
            last_read_path: None,
            config_notice: None,
        }
    }
    pub fn increment_loading_progress_indicator(&mut self) {
        // increasing and decreasing this number will increase
        // the scanning text animation speed
        self.loading_progress_indicator += 3;
    }
}
