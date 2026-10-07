use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_jett"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn dry_run_flags_and_all_delimiters_parse_with_existing_options() {
    for delimiter in ["newline", "nul", "tab", "pipe"] {
        // Theme validation occurs after successful CLI parsing, before any TUI startup.
        let result = run(&[
            "--dry-run",
            "--file-list-delim",
            delimiter,
            "--apparent-size",
            "--disable-delete-confirmation",
            "--theme",
            "missing",
            "/tmp",
        ]);
        assert_eq!(result.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("unknown theme scheme 'missing'"),
            "{result:?}"
        );
    }
    let result = run(&["--dry-run", "--theme", "missing"]);
    assert!(String::from_utf8_lossy(&result.stderr).contains("unknown theme scheme 'missing'"));
}

#[test]
fn list_delimiter_requires_dry_run_and_rejects_unknown_values() {
    for delimiter in ["newline", "nul", "tab", "pipe"] {
        let result = run(&["--file-list-delim", delimiter]);
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains("--dry-run"));
    }
    let result = run(&["--dry-run", "--file-list-delim", "comma"]);
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("invalid value"));
}

#[test]
fn help_explains_the_unix_delimiter_contract() {
    let result = run(&["--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    for text in [
        "--dry-run",
        "--file-list-delim",
        "cannot appear inside a Unix filename",
        "newline, tab, and pipe",
    ] {
        assert!(help.contains(text), "{help}");
    }
}
