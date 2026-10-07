use super::*;
use crate::tests::cases::test_utils::test_backend_factory;
use std::ffi::OsString;
use std::sync::mpsc;

#[test]
fn composition_phase_is_scan_state_not_overlay_mode() {
    let deletion = FileToDelete {
        path_in_filesystem: PathBuf::from("/synthetic"),
        path_to_file: vec!["parent".into()],
        file_type: crate::state::tiles::FileType::Folder,
        num_descendants: Some(2),
        size: 100,
    };
    for (loaded, errors, mode, expected) in [
        (false, 0, UiMode::Normal, "~2 seen; % of folder"),
        (
            false,
            0,
            UiMode::Exiting { app_loaded: false },
            "~2 seen; % of folder",
        ),
        (
            false,
            1,
            UiMode::WarningMessage(deletion.clone()),
            "~2 seen; % of folder",
        ),
        (true, 1, UiMode::Normal, "~2 seen; % of folder"),
        (
            true,
            1,
            UiMode::DeleteFile(deletion.clone()),
            "~2 seen; % of folder",
        ),
        (true, 0, UiMode::Loading, "2 items; % of folder"),
        (
            true,
            0,
            UiMode::Exiting { app_loaded: true },
            "2 items; % of folder",
        ),
    ] {
        let (_, draws, backend) = test_backend_factory(120, 40);
        let (sender, _) = mpsc::sync_channel(1);
        let path = PathBuf::from("/synthetic");
        let mut app = App::new(backend, path.clone(), sender, true, false, Theme::default());
        let mut root = Folder::from(OsString::from("root"));
        root.add_file(PathBuf::from("parent/a"), 60);
        root.add_file(PathBuf::from("parent/b"), 40);
        app.file_tree = ManuallyDrop::new(FileTree::new(root, path, true));
        app.file_tree.failed_to_read = errors;
        app.loaded = loaded;
        app.ui_mode = mode;
        app.render_and_update_board();
        let frames = draws.lock().unwrap();
        assert!(frames.last().unwrap().contains(expected));
    }
}
