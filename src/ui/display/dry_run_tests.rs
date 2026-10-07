use super::*;
use crate::state::FileToDelete;
use crate::state::files::Folder;
use crate::state::theme_selector::{ThemeChoice, ThemeSelector};
use crate::state::tiles::FileType;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

fn row(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn mode_chip_and_counter_match_all_flag_combinations_in_every_ui_mode() {
    for theme_name in crate::ui::theme::BUILTIN_SCHEMES {
        let theme = Theme::named(theme_name).unwrap();
        for (dry, confirm_off, text, color) in [
            (true, false, "DRY-RUN ON", theme.success),
            (true, true, "DRY-RUN ON", theme.success),
            (false, false, "DRY-RUN OFF", theme.warning),
            (false, true, "DRY-RUN OFF + CONFIRM OFF", theme.error),
        ] {
            for width in [50, 190] {
                let path = PathBuf::from("/synthetic");
                let mut folder = Folder::new(&path);
                folder.add_file(PathBuf::from("file"), 2048);
                let mut board = Board::new(&folder);
                let mut tree = FileTree::new(folder, path, true);
                tree.space_freed = 1024;
                let mut display = Display::new(TestBackend::new(width, 30), theme);
                let mut effects = UiEffects::new();
                effects.dry_run = dry;
                effects.delete_confirmation_disabled = confirm_off;
                let file = FileToDelete {
                    path_in_filesystem: PathBuf::from("/synthetic"),
                    path_to_file: vec!["file".into()],
                    file_type: FileType::File,
                    num_descendants: None,
                    size: 2048,
                };
                for mode in [
                    UiMode::Loading,
                    UiMode::Normal,
                    UiMode::ThemeSelector(ThemeSelector::new(
                        vec![ThemeChoice {
                            name: "default".into(),
                            theme,
                        }],
                        theme,
                        "default".into(),
                    )),
                    UiMode::DeleteFile(file),
                    UiMode::ErrorMessage("failure".into()),
                    UiMode::Exiting { app_loaded: true },
                    UiMode::ScreenTooSmall,
                ] {
                    display.render(
                        &mut tree,
                        &mut board,
                        &mode,
                        &effects,
                        CompositionPhase::Complete,
                    );
                    let buffer = display.test_backend().buffer();
                    let bottom = row(buffer, 29);
                    assert!(bottom.ends_with(&format!("{text} ")), "{bottom}");
                    let x = width - text.len() as u16 - 1;
                    assert_eq!(buffer[(x, 29)].fg, color);
                    assert_eq!(buffer[(x, 29)].modifier, Modifier::BOLD);
                    if width == 190 && matches!(mode, UiMode::Normal) {
                        let title = row(buffer, 0);
                        assert_eq!(title.contains("would free: 1.0K"), dry);
                        assert_eq!(title.contains("freed: 1.0K"), !dry);
                    }
                    if matches!(mode, UiMode::DeleteFile(_)) {
                        let all: String = (0..30).map(|y| row(buffer, y)).collect();
                        assert_eq!(all.contains("DRY-RUN: "), dry);
                        assert!(all.contains("(y/n)"));
                        effects.deletion_in_progress = true;
                        display.render(
                            &mut tree,
                            &mut board,
                            &mode,
                            &effects,
                            CompositionPhase::Complete,
                        );
                        let buffer = display.test_backend().buffer();
                        let all: String = (0..30).map(|y| row(buffer, y)).collect();
                        assert_eq!(all.contains("Planning deletion"), dry);
                        assert_eq!(all.contains("DRY-RUN: "), dry);
                        effects.deletion_in_progress = false;
                    }
                }
            }
        }
    }
}
