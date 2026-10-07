use super::*;
use ratatui::style::Color;

fn parse(text: &str) -> Result<Config> {
    let config: Config = toml::from_str(text)?;
    config.validate()?;
    Ok(config)
}

#[test]
fn unix_paths_use_xdg_then_home_even_on_macos() {
    assert_eq!(
        unix_path(Some("/xdg".into()), Some("/home/user".into())).unwrap(),
        PathBuf::from("/xdg/jett/config.toml")
    );
    for xdg in [None, Some("".into())] {
        assert_eq!(
            unix_path(xdg, Some("/home/user".into())).unwrap(),
            PathBuf::from("/home/user/.config/jett/config.toml")
        );
    }
    assert_eq!(
        unix_path(Some("/xdg".into()), None).unwrap(),
        PathBuf::from("/xdg/jett/config.toml")
    );
    assert!(unix_path(None, None).is_err());
    assert!(unix_path(None, Some("".into())).is_err());
}

#[test]
fn windows_path_uses_appdata() {
    assert_eq!(
        windows_path(Some("roaming".into())).unwrap(),
        PathBuf::from("roaming").join("jett").join("config.toml")
    );
    assert!(windows_path(None).is_err());
    assert!(windows_path(Some("".into())).is_err());
}

#[test]
fn no_theme_preserves_default_and_flag_overrides_config() {
    for config in [Config::default(), parse("version = 1").unwrap()] {
        let selection = select_theme(None, Ok(config)).unwrap();
        assert_eq!(selection.theme, Theme::default());
        assert!(selection.notice.is_none());
    }
    let text = "version = 1\n[theme]\nscheme = 'gruvbox-light'";
    assert_eq!(
        select_theme(None, parse(text)).unwrap().theme,
        Theme::named("gruvbox-light").unwrap()
    );
    assert_eq!(
        select_theme(Some("dracula"), parse(text)).unwrap().theme,
        Theme::named("dracula").unwrap()
    );
    assert_eq!(
        select_theme(Some("default"), parse(text)).unwrap().theme,
        Theme::default()
    );
}

#[test]
fn invalid_config_falls_back_with_a_notice_and_valid_flag_still_wins() {
    let invalid = "version = 1\n[theme]\nscheme = 'typo'";
    let selection = select_theme(None, parse(invalid)).unwrap();
    assert_eq!(selection.theme, Theme::default());
    let notice = selection.notice.unwrap();
    assert!(notice.contains("Config ignored:"));
    assert!(notice.contains("unknown theme scheme 'typo'"));
    assert!(notice.contains("jett config validate"));
    let selection = select_theme(Some("light"), parse(invalid)).unwrap();
    assert_eq!(selection.theme, Theme::named("light").unwrap());
    assert!(selection.notice.is_some());
    assert!(select_theme(None, Err(anyhow::anyhow!("unreadable config"))).is_ok());
    assert!(select_theme(Some("missing"), Ok(Config::default())).is_err());
}

#[test]
fn custom_colors_overlay_the_mode_base_without_changing_other_roles() {
    let config = parse(
        r##"version = 1
[theme]
scheme = "personal"
[theme.custom_schemes.personal]
foreground = "#abc"
title = "#123456"
accent = "#fedcba00"
success = { rgb = [1, 2, 255] }
error = "LIGHT_RED"
warning = "none"
selected_file_fg = "reset"
selected_file_bg = "dark gray"
selected_folder_line1_fg = "white"
selected_folder_line2_fg = "grey"
"##,
    )
    .unwrap();
    let theme = select_theme(None, Ok(config)).unwrap().theme;
    let mut expected = Theme::named("dark").unwrap();
    expected.foreground = Color::Rgb(0xaa, 0xbb, 0xcc);
    expected.title = Color::Rgb(0x12, 0x34, 0x56);
    expected.accent = Color::Rgb(0xfe, 0xdc, 0xba);
    expected.success = Color::Rgb(1, 2, 255);
    expected.error = Color::LightRed;
    expected.warning = Color::Reset;
    expected.selected_file.fg = Color::Reset;
    expected.selected_file.bg = Color::DarkGray;
    expected.selected_folder.line1_fg = Color::White;
    expected.selected_folder.line2_fg = Color::Gray;
    assert_eq!(theme, expected);
}

#[test]
fn modes_are_explicit_then_inferred_from_custom_names() {
    for (name, mode, expected) in [
        ("personal", "", "dark"),
        ("personal-light", "", "light"),
        ("LATTE", "", "light"),
        ("personal-light", "mode = 'dark'", "dark"),
        ("personal", "mode = 'light'", "light"),
    ] {
        let text = format!("version = 1\n[theme.custom_schemes.{name}]\n{mode}");
        assert_eq!(
            parse(&text).unwrap().resolve(name).unwrap(),
            Theme::named(expected).unwrap(),
            "{name}: {mode}"
        );
    }
    let text =
        "version = 1\n[theme]\nmode = 'light'\n[theme.custom_schemes.personal]\nmode = 'dark'";
    assert_eq!(
        parse(text).unwrap().resolve("personal").unwrap(),
        Theme::named("light").unwrap()
    );
}

#[test]
fn custom_names_can_be_selected_by_flag() {
    let text =
        "version = 1\n[theme]\nscheme = 'dracula'\n[theme.custom_schemes.personal]\nmode = 'light'";
    assert_eq!(
        select_theme(Some("personal"), parse(text)).unwrap().theme,
        Theme::named("light").unwrap()
    );
}

#[test]
fn malformed_configs_report_the_specific_problem() {
    for (text, expected) in [
        ("version = 2", "unsupported config version 2"),
        ("[theme]\nscheme = 'dark'", "missing field `version`"),
        ("version = 1\nunknown = true", "unknown field `unknown`"),
        (
            "version = 1\n[theme]\nmode = 'auto'",
            "unknown variant `auto`",
        ),
        (
            "version = 1\n[theme]\nscheme = 'unknown'",
            "unknown theme scheme 'unknown'",
        ),
        (
            "version = 1\n[theme]\nscheem = 'dark'",
            "unknown field `scheem`",
        ),
        (
            "version = 1\n[theme.custom_schemes.dark]",
            "conflicts with a built-in",
        ),
        (
            "version = 1\n[theme.custom_schemes.'']",
            "names must be nonempty",
        ),
        (
            "version = 1\n[theme.custom_schemes.personal]\nmode = 'auto'",
            "personal.mode: expected 'dark' or 'light'",
        ),
        (
            "version = 1\n[theme.custom_schemes.personal]\nunknown = 'red'",
            "personal.unknown: unknown color role 'unknown'",
        ),
    ] {
        let error = format!("{:#}", parse(text).unwrap_err());
        assert!(error.contains(expected), "{text}: {error}");
    }
    assert!(parse("this is not TOML").is_err());
}

#[test]
fn invalid_colors_include_the_scheme_and_role_even_when_unused() {
    for value in [
        "'chartreuse'",
        "'#ab'",
        "'#abcd'",
        "'#gggggg'",
        "'#abcdefgg'",
        "'#é12345'",
        "12",
        "true",
        "[1, 2, 3]",
        "{ rgb = [1, 2] }",
        "{ rgb = [1, 2, 256] }",
        "{ rgb = [-1, 2, 3] }",
        "{ rgb = [1, 2.0, 3] }",
        "{ rgb = [1, 2, '3'] }",
        "{ rgb = [1, 2, 3], extra = 4 }",
    ] {
        let text = format!("version = 1\n[theme.custom_schemes.personal]\naccent = {value}");
        let error = format!("{:#}", parse(&text).unwrap_err());
        assert!(
            error.contains("theme.custom_schemes.personal.accent"),
            "{error}"
        );
        assert!(error.contains("invalid color"), "{error}");
    }
}

#[test]
fn every_named_ratatui_color_is_supported() {
    for (name, expected) in [
        ("reset", Color::Reset),
        ("none", Color::Reset),
        ("black", Color::Black),
        ("red", Color::Red),
        ("green", Color::Green),
        ("yellow", Color::Yellow),
        ("blue", Color::Blue),
        ("magenta", Color::Magenta),
        ("cyan", Color::Cyan),
        ("gray", Color::Gray),
        ("darkgray", Color::DarkGray),
        ("lightred", Color::LightRed),
        ("lightgreen", Color::LightGreen),
        ("lightyellow", Color::LightYellow),
        ("lightblue", Color::LightBlue),
        ("lightmagenta", Color::LightMagenta),
        ("lightcyan", Color::LightCyan),
        ("white", Color::White),
    ] {
        let text = format!("version = 1\n[theme.custom_schemes.personal]\naccent = '{name}'");
        assert_eq!(
            parse(&text).unwrap().resolve("personal").unwrap().accent,
            expected
        );
    }
}
