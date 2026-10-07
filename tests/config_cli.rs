use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(format!(
            "/tmp/jett_tests/config_cli/{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn config_path(&self) -> PathBuf {
        self.0.join("jett/config.toml")
    }

    fn write(&self, text: &str) {
        fs::create_dir_all(self.config_path().parent().unwrap()).unwrap();
        fs::write(self.config_path(), text).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_jett"))
            .args(args)
            .env("XDG_CONFIG_HOME", &self.0)
            .env("APPDATA", &self.0)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn missing_config_validates_without_creating_a_file() {
    let fixture = Fixture::new();
    let result = fixture.run(&["config", "validate"]);
    assert!(result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stdout).contains("Config valid:"));
    assert!(!fixture.config_path().exists());
}

#[test]
fn set_creates_parent_dirs_and_round_trips_builtin_selection() {
    let fixture = Fixture::new();
    let result = fixture.run(&["config", "set", "theme.scheme", "gruvbox-light"]);
    assert!(result.status.success(), "{result:?}");
    let text = fs::read_to_string(fixture.config_path()).unwrap();
    let config: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(config["version"].as_integer(), Some(1));
    assert_eq!(config["theme"]["scheme"].as_str(), Some("gruvbox-light"));
    assert!(fixture.run(&["config", "validate"]).status.success());
}

#[test]
fn set_preserves_custom_palettes_and_mode_but_not_comments() {
    let fixture = Fixture::new();
    fixture.write(
        "# personal palette\nversion = 1\n[theme]\nscheme = 'default'\nmode = 'light'\n[theme.custom_schemes.personal]\naccent = { rgb = [1, 2, 3] }\nselected_file_bg = '#abc'\n[theme.custom_schemes.other]\nmode = 'dark'\n",
    );
    let before: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.config_path()).unwrap()).unwrap();
    let result = fixture.run(&["config", "set", "theme.scheme", "personal"]);
    assert!(result.status.success(), "{result:?}");
    let text = fs::read_to_string(fixture.config_path()).unwrap();
    assert!(!text.contains("# personal palette"));
    let after: toml::Value = toml::from_str(&text).unwrap();
    assert_eq!(after["theme"]["scheme"].as_str(), Some("personal"));
    assert_eq!(after["theme"]["mode"], before["theme"]["mode"]);
    assert_eq!(
        after["theme"]["custom_schemes"],
        before["theme"]["custom_schemes"]
    );
    assert!(fixture.run(&["config", "validate"]).status.success());
}

#[test]
fn validate_reports_invalid_colors_with_precise_key_and_nonzero_exit() {
    let fixture = Fixture::new();
    fixture.write("version = 1\n[theme.custom_schemes.personal]\naccent = { rgb = [300, 0, 0] }");
    let result = fixture.run(&["config", "validate"]);
    assert_eq!(result.status.code(), Some(2));
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(
        error.contains("theme.custom_schemes.personal.accent"),
        "{error}"
    );
    assert!(error.contains("integer channels from 0 to 255"), "{error}");
}

#[test]
fn validate_reports_toml_location_and_unknown_selection() {
    let fixture = Fixture::new();
    fixture.write("version = 1\n[theme]\nscheme = [");
    let result = fixture.run(&["config", "validate"]);
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("could not parse config"), "{error}");
    assert!(error.contains("line 3"), "{error}");
    fixture.write("version = 1\n[theme]\nscheme = 'missing'");
    let result = fixture.run(&["config", "validate"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown theme scheme 'missing'"));
    // A valid set command repairs an invalid selected name without discarding the file.
    assert!(
        fixture
            .run(&["config", "set", "theme.scheme", "dark"])
            .status
            .success()
    );
    assert!(fixture.run(&["config", "validate"]).status.success());
}

#[test]
fn rejected_sets_do_not_modify_the_existing_config() {
    let fixture = Fixture::new();
    let original = "# keep this\nversion = 1\n[theme]\nscheme = 'dark'";
    fixture.write(original);
    for args in [
        ["config", "set", "theme.scheme", "missing"],
        ["config", "set", "unknown.key", "light"],
    ] {
        assert!(!fixture.run(&args).status.success());
        assert_eq!(fs::read_to_string(fixture.config_path()).unwrap(), original);
    }
    let malformed = "not TOML";
    fixture.write(malformed);
    assert!(
        !fixture
            .run(&["config", "set", "theme.scheme", "light"])
            .status
            .success()
    );
    assert_eq!(
        fs::read_to_string(fixture.config_path()).unwrap(),
        malformed
    );
}

#[cfg(unix)]
#[test]
fn set_preserves_existing_config_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    fixture.write("version = 1\n[theme]\nscheme = 'default'\n");
    let path = fixture.config_path();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

    let result = fixture.run(&["config", "set", "theme.scheme", "light"]);
    assert!(result.status.success(), "{result:?}");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let config: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(config["theme"]["scheme"].as_str(), Some("light"));
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn failed_set_preserves_original_config_bytes() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let original = "# personal palette\nversion = 1\n[theme]\nscheme = 'default'\n[theme.custom_schemes.personal]\naccent = '#abc'\n";
    fixture.write(original);
    let path = fixture.config_path();
    let parent = path.parent().unwrap();
    let permissions = fs::metadata(parent).unwrap().permissions();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o555)).unwrap();
    let result = fixture.run(&["config", "set", "theme.scheme", "personal"]);
    fs::set_permissions(parent, permissions).unwrap();

    assert!(!result.status.success(), "{result:?}");
    assert!(String::from_utf8_lossy(&result.stderr).contains("could not write config"));
    assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
    assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
}

#[test]
fn unknown_theme_flag_errors_before_starting_the_tui() {
    let fixture = Fixture::new();
    let result = fixture.run(&["--theme", "missing"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown theme scheme 'missing'"));
}

#[test]
fn existing_navigation_flags_and_subcommand_help_parse() {
    let fixture = Fixture::new();
    for args in [
        vec![
            "--apparent-size",
            "--disable-delete-confirmation",
            "--theme",
            "light",
            "--help",
        ],
        vec!["config", "--help"],
        vec!["config", "set", "--help"],
    ] {
        assert!(fixture.run(&args).status.success(), "{args:?}");
    }
}
