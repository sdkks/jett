use super::*;
use crate::state::FileToDelete;
use crate::state::files::Folder;
use crate::state::tiles::{FileType, Tile};
use crate::ui::grid::tile_style;
use crate::ui::theme::BUILTIN_SCHEMES;
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier, Style};

fn fixture() -> (FileTree, Board) {
    // Entirely synthetic metadata: this fixture never accesses the filesystem.
    let path = PathBuf::from("/synthetic");
    let mut folder = Folder::new(&path);
    folder.add_file(PathBuf::from("file"), 8192);
    folder.add_folder(PathBuf::from("folder"));
    folder.add_file(PathBuf::from("folder/child"), 4096);
    let board = Board::new(&folder);
    (FileTree::new(folder, path, true), board)
}

fn text_cell<'a>(buffer: &'a Buffer, text: &str) -> &'a Cell {
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        if let Some(byte) = line.find(text) {
            let x = line[..byte].chars().count() as u16;
            return &buffer[(x, y)];
        }
    }
    panic!("missing rendered text: {text}");
}

fn assert_style(cell: &Cell, fg: Color, bg: Color, modifier: Modifier) {
    assert_eq!((cell.fg, cell.bg, cell.modifier), (fg, bg, modifier));
}

#[test]
fn every_scheme_reaches_all_display_modes() {
    for name in BUILTIN_SCHEMES {
        let theme = Theme::named(name).unwrap();
        let mut display = Display::new(TestBackend::new(190, 50), theme);
        let (mut tree, mut board) = fixture();
        tree.failed_to_read = 3;
        let mut effects = UiEffects::new();
        effects.loading_progress_indicator = 10;
        effects.last_read_path = Some(PathBuf::from("/last-read"));
        let file = FileToDelete {
            path_in_filesystem: PathBuf::from("/synthetic"),
            path_to_file: vec!["file".into()],
            file_type: FileType::File,
            num_descendants: None,
            size: 8192,
        };
        for (mode, text, fg, bg, modifier) in [
            (
                UiMode::Normal,
                "Total:",
                theme.title,
                Color::Reset,
                Modifier::BOLD,
            ),
            (
                UiMode::Loading,
                "Scanning:",
                theme.title,
                Color::Reset,
                Modifier::empty(),
            ),
            (
                UiMode::DeleteFile(file.clone()),
                "Delete this file?",
                theme.error,
                theme.modal_surface,
                Modifier::BOLD,
            ),
            (
                UiMode::ErrorMessage("themed failure".into()),
                "themed failure",
                theme.error,
                theme.modal_surface,
                Modifier::BOLD,
            ),
            (
                UiMode::Exiting { app_loaded: true },
                "Are you sure",
                theme.modal_text,
                theme.modal_surface,
                Modifier::BOLD,
            ),
            (
                UiMode::Exiting { app_loaded: false },
                "Are you sure",
                theme.modal_text,
                theme.modal_surface,
                Modifier::BOLD,
            ),
            (
                UiMode::WarningMessage(file.clone()),
                "Sorry, deletion",
                theme.warning,
                theme.modal_surface,
                Modifier::BOLD,
            ),
            (
                UiMode::ScreenTooSmall,
                "Terminal window",
                Color::Reset,
                Color::Reset,
                Modifier::BOLD,
            ),
        ] {
            display.render(
                &mut tree,
                &mut board,
                &mode,
                &effects,
                CompositionPhase::Partial,
            );
            assert_style(
                text_cell(display.terminal.backend().buffer(), text),
                fg,
                bg,
                modifier,
            );
        }
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Loading,
            &effects,
            CompositionPhase::Scanning,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "/last-read"),
            theme.foreground,
            Color::Reset,
            Modifier::empty(),
        );
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &effects,
            CompositionPhase::Partial,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "failed to read"),
            theme.error,
            Color::Reset,
            Modifier::BOLD,
        );
        board.unrenderable_tile_coordinates = Some((160, 25));
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &effects,
            CompositionPhase::Partial,
        );
        let buffer = display.terminal.backend().buffer();
        assert_style(
            text_cell(buffer, "Small files"),
            theme.foreground,
            Color::Reset,
            Modifier::empty(),
        );
        assert_style(
            &buffer[(173, 48)],
            theme.legend_chip.fg,
            theme.legend_chip.bg,
            Modifier::empty(),
        );
        assert_style(
            &buffer[(161, 26)],
            theme.legend_chip.fg,
            theme.legend_chip.bg,
            Modifier::empty(),
        );
        board.unrenderable_tile_coordinates = None;

        effects.deletion_in_progress = true;
        display.render(
            &mut tree,
            &mut board,
            &UiMode::DeleteFile(file),
            &effects,
            CompositionPhase::Partial,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "Deleting"),
            theme.error,
            theme.modal_surface,
            Modifier::BOLD,
        );
        effects.deletion_in_progress = false;
        effects.flash_space_freed = true;
        effects.current_path_is_red = true;
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &effects,
            CompositionPhase::Partial,
        );
        let buffer = display.terminal.backend().buffer();
        assert_style(
            text_cell(buffer, "Total:"),
            theme.freed_flash.fg,
            theme.freed_flash.bg,
            Modifier::empty(),
        );
        assert_style(
            text_cell(buffer, "/synthetic"),
            theme.path_error.fg,
            theme.path_error.bg,
            Modifier::empty(),
        );
        assert_style(
            text_cell(buffer, " | "),
            theme.title_separator,
            Color::Reset,
            Modifier::BOLD,
        );

        effects.flash_space_freed = false;
        effects.current_path_is_red = false;
        for file_type in [FileType::File, FileType::Folder] {
            board.selected_index = board.tiles.iter().position(|t| t.file_type == file_type);
            display.render(
                &mut tree,
                &mut board,
                &UiMode::Normal,
                &effects,
                CompositionPhase::Partial,
            );
            let buffer = display.terminal.backend().buffer();
            let status_fg = if file_type == FileType::Folder {
                theme.accent
            } else {
                theme.foreground
            };
            assert_style(
                text_cell(buffer, "SELECTED:"),
                status_fg,
                Color::Reset,
                Modifier::BOLD,
            );
            let tile = board.currently_selected().unwrap();
            let text = if file_type == FileType::Folder {
                "folder/"
            } else {
                "file"
            };
            let (fg, bg) = if file_type == FileType::Folder {
                (theme.selected_folder.line1_fg, theme.selected_folder.bg)
            } else {
                (theme.selected_file.fg, theme.selected_file.bg)
            };
            // Search inside the selected tile, excluding title/status occurrences.
            let cell = (tile.y + 1..tile.y + tile.height)
                .find_map(|y| {
                    let line: String = (tile.x + 1..tile.x + tile.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect();
                    line.find(text)
                        .map(|byte| &buffer[(tile.x + 1 + line[..byte].chars().count() as u16, y)])
                })
                .unwrap();
            assert_style(cell, fg, bg, Modifier::BOLD);
            assert!(tile.height > 2);
        }

        tree.enter_folder(std::ffi::OsStr::new("folder"));
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &effects,
            CompositionPhase::Partial,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "/folder"),
            theme.success,
            Color::Reset,
            Modifier::BOLD,
        );

        let path = PathBuf::from("/empty");
        let empty = Folder::new(&path);
        let mut board = Board::new(&empty);
        let mut tree = FileTree::new(empty, path, true);
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &effects,
            CompositionPhase::Complete,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "Folder is empty"),
            theme.empty_surface.fg,
            theme.empty_surface.bg,
            Modifier::empty(),
        );
    }
}

#[test]
fn default_tile_styles_are_exact_legacy_patches() {
    let theme = Theme::default();
    let mut tile = Tile {
        x: 0,
        y: 0,
        width: 40,
        height: 10,
        name: "file".into(),
        size: 8192,
        descendants: None,
        percentage: 1.0,
        file_type: FileType::File,
        composition: None,
    };
    let file_text = Style::default()
        .fg(Color::Magenta)
        .bg(Color::Gray)
        .add_modifier(Modifier::BOLD);
    assert_eq!(
        tile_style(&tile, true, &theme),
        (
            Some(Style::default().fg(Color::Gray).bg(Color::Gray)),
            file_text,
            file_text
        )
    );
    assert_eq!(
        tile_style(&tile, false, &theme),
        (None, Style::default(), Style::default())
    );
    tile.file_type = FileType::Folder;
    assert_eq!(
        tile_style(&tile, true, &theme),
        (
            Some(Style::default().fg(Color::Blue).bg(Color::Blue)),
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::Black).bg(Color::Blue),
        )
    );
    assert_eq!(
        tile_style(&tile, false, &theme),
        (
            None,
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            Style::default(),
        )
    );
}

#[test]
fn configured_and_flag_selected_palettes_reach_the_rendered_ui() {
    let text = "version = 1\n[theme]\nscheme = 'personal'\n[theme.custom_schemes.personal]\nmode = 'light'\ntitle = '#123456'";
    for (flag, expected_title) in [
        (None, Color::Rgb(0x12, 0x34, 0x56)),
        (Some("default"), Color::Yellow),
    ] {
        let config = toml::from_str(text).unwrap();
        let selection = crate::config::select_theme(flag, Ok(config)).unwrap();
        let mut display = Display::new(TestBackend::new(190, 50), selection.theme);
        let (mut tree, mut board) = fixture();
        display.render(
            &mut tree,
            &mut board,
            &UiMode::Normal,
            &UiEffects::new(),
            CompositionPhase::Complete,
        );
        assert_style(
            text_cell(display.terminal.backend().buffer(), "Total:"),
            expected_title,
            Color::Reset,
            Modifier::BOLD,
        );
    }
}

#[test]
fn config_notice_is_non_modal_and_visible_during_scanning_and_navigation() {
    let selection =
        crate::config::select_theme(None, Err(anyhow::anyhow!("bad config\naccent"))).unwrap();
    let theme = selection.theme;
    let mut effects = UiEffects::new();
    effects.config_notice = selection.notice;
    for width in [50, 190] {
        let mut display = Display::new(TestBackend::new(width, 30), theme);
        let (mut tree, mut board) = fixture();
        for mode in [UiMode::Loading, UiMode::Normal] {
            display.render(
                &mut tree,
                &mut board,
                &mode,
                &effects,
                CompositionPhase::Scanning,
            );
            let buffer = display.terminal.backend().buffer();
            assert_style(
                text_cell(buffer, "Config ignored:"),
                theme.warning,
                Color::Reset,
                Modifier::BOLD,
            );
            if width == 50 {
                text_cell(buffer, "navigate");
            } else {
                text_cell(buffer, "<arrows> - move around");
                text_cell(buffer, "bad config accent");
            }
        }
    }
}
