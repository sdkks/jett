use super::*;
use crate::input::{
    handle_keypress_loading_mode, handle_keypress_normal_mode, handle_keypress_theme_selector,
};
use crossterm::event::{Event as InputEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use unicode_width::UnicodeWidthStr;

fn fixture(name: &str, width: u16, config: &str, flag: Option<&str>) -> App<TestBackend> {
    let path = PathBuf::from(format!("/tmp/jett_tests/theme_selector/{name}/config.toml"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, config).unwrap();
    let selection = crate::config::select_theme(flag, Config::load(&path)).unwrap();
    let (sender, _) = std::sync::mpsc::sync_channel(100);
    let mut app = App::new(
        TestBackend::new(width, 20),
        PathBuf::from("/synthetic"),
        sender,
        true,
        false,
        selection.theme,
    );
    app.config_path = Ok(path);
    app.configure_theme(selection);
    let mut folder = Folder::new(&PathBuf::from("/synthetic"));
    folder.add_file(PathBuf::from("file"), 8192);
    folder.add_folder(PathBuf::from("folder"));
    folder.add_file(PathBuf::from("folder/child"), 4096);
    app.file_tree = ManuallyDrop::new(FileTree::new(folder, PathBuf::from("/synthetic"), true));
    app.start_ui();
    app
}

fn key(app: &mut App<TestBackend>, code: KeyCode) {
    let event = InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE));
    if matches!(app.ui_mode, UiMode::Normal) {
        handle_keypress_normal_mode(event, app);
    } else {
        handle_keypress_theme_selector(event, app);
    }
}

fn filter(app: &mut App<TestBackend>, text: &str) {
    for character in text.chars() {
        key(app, KeyCode::Char(character));
    }
}

fn selector(app: &App<TestBackend>) -> &ThemeSelector {
    let UiMode::ThemeSelector(selector) = &app.ui_mode else {
        panic!("selector closed")
    };
    selector
}

fn buffer(app: &mut App<TestBackend>) -> Buffer {
    app.display.test_backend().buffer().clone()
}

fn screen(app: &mut App<TestBackend>) -> String {
    let buffer = buffer(app);
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += symbol.width().max(1) as u16;
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn filter_arrows_preview_and_escape_restore_exact_runtime_without_writes() {
    let config = "version = 1\n[theme]\nscheme = 'light'\n[theme.custom_schemes.personal]\ntitle = '#123456'\n";
    let mut app = fixture("escape", 100, config, Some("dracula"));
    let path = app.config_path.as_ref().unwrap().clone();
    let original = app.display.theme();
    let before = buffer(&mut app);
    key(&mut app, KeyCode::Char('t'));
    assert_eq!(selector(&app).choices.len(), 21);
    filter(&mut app, "solarized");
    assert_eq!(selector(&app).matches.len(), 2);
    assert_eq!(app.display.theme(), Theme::named("solarized-dark").unwrap());
    let preview = buffer(&mut app);
    assert_ne!(preview[(1, 0)].fg, before[(1, 0)].fg);
    key(&mut app, KeyCode::Down);
    assert_eq!(
        app.display.theme(),
        Theme::named("solarized-light").unwrap()
    );
    let light_preview = buffer(&mut app);
    assert!((1..18).any(|y| (44..100).any(|x| {
        !preview[(x, y)].symbol().trim().is_empty()
            && light_preview[(x, y)].fg != preview[(x, y)].fg
    })));
    key(&mut app, KeyCode::Up);
    assert_eq!(app.display.theme(), Theme::named("solarized-dark").unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), config);
    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert_eq!(app.display.theme(), original);
    assert_eq!(app.theme_name, "dracula");
    assert_eq!(buffer(&mut app), before);
    assert_eq!(fs::read_to_string(path).unwrap(), config);
}

#[test]
fn enter_persists_custom_selection_and_preserves_config() {
    let config = "version = 1\n[theme]\nscheme = 'default'\nmode = 'light'\n[theme.custom_schemes.personal]\ntitle = '#123456'\n";
    let mut app = fixture("save", 100, config, None);
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "PERSONAL");
    let expected = app.display.theme();
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert!(app.is_running);
    assert_eq!(app.theme_name, "personal");
    assert_eq!(app.display.theme(), expected);
    let path = app.config_path.as_ref().unwrap();
    let saved = Config::load(path).unwrap();
    let selection = crate::config::select_theme(None, Ok(saved)).unwrap();
    assert_eq!(selection.name, "personal");
    assert_eq!(selection.theme, expected);
    assert_eq!(selection.config.scheme_names().len(), 21);
    assert_eq!(
        selection.config.resolve("dark").unwrap().mode,
        crate::ui::theme::ThemeMode::Light
    );
    let bytes = fs::read(path).unwrap();
    key(&mut app, KeyCode::Char('t'));
    assert_eq!(selector(&app).selected().unwrap().name, "personal");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.display.theme(), expected);
    assert_eq!(fs::read(app.config_path.as_ref().unwrap()).unwrap(), bytes);
}

#[test]
fn failed_save_restores_runtime_and_displays_non_fatal_notice() {
    let mut app = fixture("failure", 100, "version = 1", Some("nord"));
    let original = app.display.theme();
    let path = app.config_path.as_ref().unwrap().clone();
    fs::create_dir_all(path.with_extension("directory")).unwrap();
    app.config_path = Ok(path.with_extension("directory"));
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "dracula");
    key(&mut app, KeyCode::Enter);
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert!(app.is_running);
    assert_eq!(app.display.theme(), original);
    assert_eq!(app.theme_name, "nord");
    assert!(screen(&mut app).contains("Theme not saved:"));
    assert_eq!(fs::read_to_string(path).unwrap(), "version = 1");
    key(&mut app, KeyCode::Char('t'));
    key(&mut app, KeyCode::Esc);
    assert!(app.is_running);
}

#[cfg(unix)]
#[test]
fn read_only_directory_save_fails_without_changing_config() {
    use std::os::unix::fs::PermissionsExt;
    let mut app = fixture("readonly", 100, "version = 1", None);
    let path = app.config_path.as_ref().unwrap().clone();
    let parent = path.parent().unwrap();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "light");
    key(&mut app, KeyCode::Enter);
    fs::set_permissions(parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(screen(&mut app).contains("Theme not saved:"));
    assert_eq!(app.display.theme(), Theme::default());
    assert_eq!(fs::read_to_string(path).unwrap(), "version = 1");
}

#[test]
fn unicode_filter_backspace_and_empty_results_are_safe() {
    let mut app = fixture(
        "unicode",
        50,
        "version = 1\n[theme.custom_schemes.'界éq']\nmode = 'light'",
        None,
    );
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "界éq");
    assert_eq!(selector(&app).selected().unwrap().name, "界éq");
    assert_eq!(selector(&app).visible_filter(2), "éq");
    assert_eq!(selector(&app).visible_filter(3).width(), 2);
    assert_eq!(selector(&app).visible_filter(4), "界éq");
    assert_eq!(selector(&app).visible_filter(0), "");
    assert_eq!(selector(&app).visible_filter(1), "q");
    key(&mut app, KeyCode::Backspace);
    assert_eq!(selector(&app).filter, "界é");
    filter(&mut app, &"界".repeat(40));
    assert!(selector(&app).visible_filter(40).width() <= 40);
    assert!(selector(&app).matches.is_empty());
    assert!(
        screen(&mut app)
            .lines()
            .nth(18)
            .unwrap()
            .trim_end()
            .ends_with('_')
    );
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Up);
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("No matching themes"));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.display.theme(), Theme::default());
}

#[test]
fn q_cancels_only_with_empty_filter_and_ctrl_c_always_cancels() {
    let mut app = fixture("quit", 100, "version = 1", None);
    key(&mut app, KeyCode::Char('t'));
    key(&mut app, KeyCode::Char('q'));
    assert!(matches!(app.ui_mode, UiMode::Normal));
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "aq");
    assert_eq!(selector(&app).filter, "aq");
    key(&mut app, KeyCode::Backspace);
    key(&mut app, KeyCode::Backspace);
    key(&mut app, KeyCode::Char('q'));
    assert!(matches!(app.ui_mode, UiMode::Normal));
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "light");
    handle_keypress_theme_selector(
        InputEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        &mut app,
    );
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert_eq!(app.display.theme(), Theme::default());
}

#[test]
fn selector_preserves_navigation_and_is_not_available_while_scanning_or_deleting() {
    let mut app = fixture("modes", 100, "version = 1", None);
    app.ui_mode = UiMode::Loading;
    handle_keypress_loading_mode(
        InputEvent::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)),
        &mut app,
    );
    assert!(matches!(app.ui_mode, UiMode::Loading));
    app.open_theme_selector();
    assert!(matches!(app.ui_mode, UiMode::Loading));
    let file = FileToDelete {
        path_in_filesystem: "/synthetic".into(),
        path_to_file: vec!["file".into()],
        file_type: crate::state::tiles::FileType::File,
        num_descendants: None,
        size: 1,
    };
    app.ui_mode = UiMode::DeleteFile(file);
    app.open_theme_selector();
    assert!(matches!(app.ui_mode, UiMode::DeleteFile(_)));
    app.normal_mode();
    let path = app.file_tree.get_current_path();
    let zoom = app.board.zoom_level;
    let selected = app.board.selected_index;
    key(&mut app, KeyCode::Char('t'));
    key(&mut app, KeyCode::Backspace);
    filter(&mut app, "hjkl+-0");
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.file_tree.get_current_path(), path);
    assert_eq!(app.board.zoom_level, zoom);
    assert_eq!(app.board.selected_index, selected);
}

#[test]
fn resize_does_not_lose_the_preview_rollback_or_escape_path() {
    let mut app = fixture("resize", 100, "version = 1", Some("nord"));
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, "light");
    app.display.test_backend().resize(20, 10);
    app.reset_ui_mode();
    app.render();
    assert!(matches!(app.ui_mode, UiMode::ThemeSelector(_)));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.display.theme(), Theme::named("nord").unwrap());
    app.display.test_backend().resize(100, 20);
    app.reset_ui_mode();
    app.render();
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert!(screen(&mut app).contains("<t> - themes") || screen(&mut app).contains("<t>: themes"));
}

#[test]
fn selector_screens() {
    let mut app = fixture(
        "screens",
        100,
        "version = 1\n[theme.custom_schemes.personal]\nmode = 'light'",
        None,
    );
    key(&mut app, KeyCode::Char('t'));
    insta::assert_snapshot!("selector_open", screen(&mut app));
    for _ in 0..25 {
        key(&mut app, KeyCode::Down);
    }
    insta::assert_snapshot!("selector_scrolled_custom", screen(&mut app));
    filter(&mut app, "solarized");
    insta::assert_snapshot!("selector_filtered", screen(&mut app));
    filter(&mut app, "not-a-theme");
    insta::assert_snapshot!("selector_no_matches", screen(&mut app));
    key(&mut app, KeyCode::Esc);
    app.display.test_backend().resize(50, 20);
    key(&mut app, KeyCode::Char('t'));
    filter(&mut app, &"界".repeat(40));
    insta::assert_snapshot!("selector_narrow_unicode", screen(&mut app));
}
