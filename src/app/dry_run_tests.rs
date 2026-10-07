use super::*;
use crate::input::handle_keypress_delete_file_mode;
use crate::state::dry_run::FileListDelimiter;
use crate::tests::cases::test_utils::test_backend_factory;
use crossterm::event::{Event as KeyEvent, KeyCode, KeyModifiers};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(format!(
            "/tmp/jett_tests/dry_run/{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("folder")).unwrap();
        fs::write(path.join("file"), vec![0; 2048]).unwrap();
        fs::write(path.join("folder/child"), vec![0; 1024]).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

type TestApp = App<crate::tests::fakes::TestBackend>;
type Frames = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

fn app(fixture: &Fixture, dry_run: bool, confirm_off: bool) -> (TestApp, Frames) {
    let (_, draws, backend) = test_backend_factory(190, 40);
    let (sender, _) = mpsc::sync_channel(10);
    let mut app = App::new(
        backend,
        fixture.0.clone(),
        sender,
        true,
        confirm_off,
        Theme::default(),
    );
    for name in ["file", "folder", "folder/child"] {
        let path = fixture.0.join(name);
        app.add_entry_to_base_folder(&fs::metadata(&path).unwrap(), path);
    }
    app.configure_dry_run(dry_run);
    app.start_ui();
    (app, draws)
}

fn select(app: &mut TestApp, name: &str) {
    app.board.selected_index = app.board.tiles.iter().position(|tile| tile.name == name);
    assert!(app.board.selected_index.is_some());
}

#[test]
fn dry_run_and_real_deletion_have_identical_tree_board_and_freed_accounting() {
    let mut outcomes = Vec::new();
    for dry in [false, true] {
        let fixture = Fixture::new();
        let (mut app, draws) = app(&fixture, dry, false);
        for name in ["file", "folder"] {
            select(&mut app, name);
            app.prompt_file_deletion();
            let UiMode::DeleteFile(file) = app.ui_mode.clone() else {
                panic!("confirmation required");
            };
            assert_eq!(
                app.file_tree.space_freed,
                if name == "file" { 0 } else { 2048 }
            );
            assert!(fixture.0.join(name).exists());
            if dry {
                assert!(
                    draws
                        .lock()
                        .unwrap()
                        .last()
                        .unwrap()
                        .contains("DRY-RUN: nothing will be deleted")
                );
            }
            handle_keypress_delete_file_mode(
                KeyEvent::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Char('y'),
                    KeyModifiers::NONE,
                )),
                &mut app,
                file,
            );
            assert_eq!(fixture.0.join(name).exists(), dry);
        }
        assert_eq!(app.file_tree.space_freed, 3072);
        assert!(app.board.tiles.is_empty());
        outcomes.push((
            app.file_tree.space_freed,
            app.file_tree.get_total_size(),
            app.file_tree.get_total_descendants(),
        ));
        if dry {
            let plan = app.take_dry_run_plan().unwrap();
            assert_eq!(
                plan.summary(),
                "dry run: would have cleaned 3.0K across 2 items"
            );
            let path = plan.finish(FileListDelimiter::Nul).unwrap().unwrap();
            let expected = format!(
                "{}\0{}\0",
                fixture.0.join("file").display(),
                fixture.0.join("folder").display()
            );
            assert_eq!(fs::read(&path).unwrap(), expected.as_bytes());
            fs::remove_file(path).unwrap();
        } else {
            assert!(app.take_dry_run_plan().is_none());
        }
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn cancelled_dry_run_deletion_does_not_account_or_list_and_confirm_off_still_works() {
    let fixture = Fixture::new();
    let (mut app, _) = app(&fixture, true, false);
    select(&mut app, "file");
    app.prompt_file_deletion();
    let UiMode::DeleteFile(file) = app.ui_mode.clone() else {
        panic!("confirmation required");
    };
    handle_keypress_delete_file_mode(
        KeyEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::NONE,
        )),
        &mut app,
        file,
    );
    assert_eq!(app.file_tree.space_freed, 0);
    assert!(fixture.0.join("file").exists());
    assert_eq!(
        app.dry_run_plan
            .as_ref()
            .unwrap()
            .finish(FileListDelimiter::Nul)
            .unwrap(),
        None
    );
    app.ui_effects.delete_confirmation_disabled = true;
    select(&mut app, "file");
    app.prompt_file_deletion();
    assert!(matches!(app.ui_mode, UiMode::Normal));
    assert_eq!(app.file_tree.space_freed, 2048);
    assert!(fixture.0.join("file").exists());
}

#[test]
fn metadata_failure_still_errors_without_accounting_or_listing_in_dry_run() {
    let fixture = Fixture::new();
    let (mut app, _) = app(&fixture, true, false);
    select(&mut app, "file");
    let file = app.get_file_to_delete().unwrap();
    fs::remove_file(file.full_path()).unwrap();
    app.delete_file(&file);
    assert!(matches!(app.ui_mode, UiMode::ErrorMessage(_)));
    assert_eq!(app.file_tree.space_freed, 0);
    assert_eq!(
        app.dry_run_plan
            .as_ref()
            .unwrap()
            .finish(FileListDelimiter::Nul)
            .unwrap(),
        None
    );
}
