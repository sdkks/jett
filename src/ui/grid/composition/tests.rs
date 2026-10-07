use super::*;
use crate::state::files::Folder;
use crate::state::tiles::{Board, ChildPreview, files_in_folder};
use crate::ui::grid::{RectangleGrid, draw_tile_text_on_grid};
use crate::ui::theme::BUILTIN_SCHEMES;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use ratatui::widgets::Widget;
use std::ffi::OsString;
use std::path::PathBuf;

fn tile(w: u16, h: u16, sizes: &[u128]) -> Tile {
    let mut folder = Folder::from(OsString::from("parent"));
    for (i, size) in sizes.iter().enumerate() {
        folder.add_file(PathBuf::from(format!("child-{i:02}")), *size);
    }
    Tile {
        x: 2,
        y: 2,
        width: w + 1,
        height: h + 1,
        name: "parent".into(),
        size: folder.size,
        descendants: Some(folder.num_descendants),
        percentage: 0.5,
        file_type: FileType::Folder,
        composition: Some(FolderComposition::from(&folder)),
    }
}

fn render(tile: &Tile, phase: CompositionPhase) -> Buffer {
    let mut buffer = Buffer::empty(Rect::new(0, 0, tile.width + 6, tile.height + 6));
    draw_tile_text_on_grid(&mut buffer, tile, false, &Theme::default(), phase);
    buffer
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| row(buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn profile_y(tile: &Tile) -> u16 {
    tile.y + 1 + tile.height - 1 - 6
}

#[test]
fn same_count_and_largest_share_have_distinct_tail_shapes() {
    // The final 9/46 are aggregates of nine entries, not fourth individual children.
    let concentrated = tile(52, 12, &[42, 40, 9, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
    let spread = tile(52, 12, &[42, 6, 6, 6, 5, 5, 5, 5, 5, 5, 5, 5]);
    let a = render(&concentrated, CompositionPhase::Complete);
    let b = render(&spread, CompositionPhase::Complete);
    let y = profile_y(&concentrated);
    assert_eq!(row(&a, y), row(&b, y));
    assert_eq!(row(&a, y + 1), row(&b, y + 1));
    assert!(row(&a, y + 2).contains("[##########................]  40%"));
    assert!(row(&b, y + 2).contains("[#.........................]   6%"));
    assert!(row(&a, y + 4).contains("9 more           [==........................]   9%"));
    assert!(row(&b, y + 4).contains("9 more           [===========...............]  46%"));
    assert_eq!(
        remainder(
            concentrated.composition.as_ref().unwrap(),
            concentrated.size
        ),
        (9, 9)
    );
    assert_eq!(
        remainder(spread.composition.as_ref().unwrap(), spread.size),
        (9, 46)
    );
}

#[test]
fn exact_interior_thresholds_and_identity_geometry() {
    for (w, h, expected) in [
        (23, 6, 0),
        (24, 5, 0),
        (23, 5, 0),
        (24, 6, 1),
        (27, 10, 1),
        (28, 9, 1),
        (27, 9, 1),
        (28, 10, 2),
        (43, 10, 2),
        (44, 10, 3),
        (60, 6, 1),
        (24, 14, 1),
    ] {
        let t = tile(w, h, &[42, 40, 9, 9]);
        let b = render(&t, CompositionPhase::Complete);
        let s = text(&b);
        match expected {
            0 => assert!(!s.contains("largest")),
            1 => {
                assert!(row(&b, t.y + h).contains("4 items, largest 42%"));
                assert!(!s.contains('['));
                assert_eq!(row(&b, t.y + h - 1).trim(), "");
            }
            2 => assert!(s.contains("largest  [")),
            3 => assert!(s.contains("#1 child-00")),
            _ => unreachable!(),
        }
        if let Some(identity) = identity_row(&t) {
            assert!(row(&b, t.y + 1 + identity).contains("parent/"));
            assert!(row(&b, t.y + 2 + identity).contains("(50%)"));
        }
        if expected >= 2 {
            assert_eq!(row(&b, profile_y(&t) - 1).trim(), "");
            assert_eq!(row(&b, t.y + h).trim(), "");
        }
        // Draw into a larger buffer to detect any writes into borders or neighbors.
        for y in 0..b.area.height {
            for x in 0..b.area.width {
                if x <= t.x || x >= t.x + t.width || y <= t.y || y >= t.y + t.height {
                    assert_eq!(b[(x, y)].symbol(), " ", "{w}x{h}: ({x},{y})");
                }
            }
        }
    }
    for (w, h) in [(0, 0), (1, 1)] {
        assert_eq!(identity_row(&tile(w, h, &[])), None);
    }
}

#[test]
fn all_phase_and_empty_zero_state_rows() {
    for (phase, sizes, summary, profile) in [
        (
            CompositionPhase::Complete,
            vec![42, 58],
            "2 items, largest 58%",
            "2 items; % of folder",
        ),
        (
            CompositionPhase::Scanning,
            vec![42, 58],
            "~2 seen, largest 58%",
            "~2 seen; % of folder",
        ),
        (
            CompositionPhase::Partial,
            vec![42, 58],
            "~2 seen, largest 58%",
            "~2 seen; % of folder",
        ),
        (
            CompositionPhase::Scanning,
            vec![],
            "~no items seen",
            "~no items seen",
        ),
        (
            CompositionPhase::Partial,
            vec![],
            "~no items seen",
            "~no items seen",
        ),
        (
            CompositionPhase::Complete,
            vec![],
            "empty (0 items)",
            "empty (0 items)",
        ),
        (
            CompositionPhase::Complete,
            vec![0, 0],
            "2 items, all 0 B",
            "2 items; all 0 B",
        ),
        (
            CompositionPhase::Scanning,
            vec![0, 0],
            "~2 seen, 0 B so far",
            "~2 seen; 0 B so far",
        ),
        (
            CompositionPhase::Partial,
            vec![0, 0],
            "~2 seen, 0 B so far",
            "~2 seen; 0 B so far",
        ),
    ] {
        for (w, h, expected) in [(24, 6, summary), (44, 10, profile)] {
            let t = tile(w, h, &sizes);
            let b = render(&t, phase);
            let y = if h == 6 { t.y + h } else { profile_y(&t) };
            assert_eq!(row(&b, y).trim(), expected);
            if t.size == 0 {
                assert!(!text(&b).contains('['));
            }
        }
    }
    assert_eq!(
        CompositionPhase::from_scan(false, 0),
        CompositionPhase::Scanning
    );
    assert_eq!(
        CompositionPhase::from_scan(false, 1),
        CompositionPhase::Scanning
    );
    assert_eq!(
        CompositionPhase::from_scan(true, 0),
        CompositionPhase::Complete
    );
    assert_eq!(
        CompositionPhase::from_scan(true, 1),
        CompositionPhase::Partial
    );
}

#[test]
fn sub_cell_positive_shares_zero_bytes_and_nearly_total_shares() {
    let t = tile(28, 10, &[9998, 1, 1, 0]);
    let b = render(&t, CompositionPhase::Complete);
    let y = profile_y(&t);
    assert!(row(&b, y + 1).contains("[#########.] >99%"));
    assert!(row(&b, y + 2).contains("[..........]  <1%"));
    assert!(row(&b, y + 3).contains("[..........]  <1%"));
    assert!(row(&b, y + 4).contains("[..........]  0 B"));
    let t = tile(28, 10, &[100, 0]);
    let b = render(&t, CompositionPhase::Complete);
    let y = profile_y(&t);
    assert!(row(&b, y + 1).contains("[##########] 100%"));
    assert!(row(&b, y + 2).contains("[..........]  0 B"));
    assert_eq!(scaled_share(u128::MAX - 1, u128::MAX, 100).0, 99);
    assert_eq!(share(u128::MAX - 1, u128::MAX), ">99%");
    assert_eq!(scaled_share(u128::MAX / 2, u128::MAX, 60).0, 29);
    assert_eq!(share(415, 1000), "42%");
    assert_eq!(share(1, 1000), "<1%");
}

#[test]
fn no_phantom_rows_and_a_many_small_items_profile() {
    for sizes in [&[100][..], &[70, 30][..], &[50, 30, 20][..]] {
        let t = tile(28, 10, sizes);
        let b = render(&t, CompositionPhase::Complete);
        let y = profile_y(&t);
        for slot in sizes.len()..4 {
            assert_eq!(row(&b, y + 1 + slot as u16).trim(), "");
        }
        assert_eq!(remainder(t.composition.as_ref().unwrap(), t.size), (0, 0));
    }
    let mut sizes = vec![400, 400, 200];
    sizes.extend(std::iter::repeat_n(95, 200));
    let t = tile(52, 12, &sizes);
    let b = render(&t, CompositionPhase::Complete);
    let y = profile_y(&t);
    assert!(row(&b, y + 1).contains("[..........................]   2%"));
    assert!(row(&b, y + 4).contains("200 more         [========================..]  95%"));
}

#[test]
fn deterministic_metadata_zoom_and_remainder_conservation() {
    let mut root = Folder::from(OsString::from("root"));
    for name in ["z", "c", "a", "b"] {
        root.add_file(PathBuf::from(format!("parent/{name}")), 10);
    }
    root.add_file(PathBuf::from("larger"), 100);
    let folder = files_in_folder(&root, 0)
        .into_iter()
        .find(|m| m.file_type == FileType::Folder)
        .unwrap();
    let zoomed = files_in_folder(&root, 1).remove(0);
    assert_ne!(folder.percentage, zoomed.percentage);
    for m in [folder, zoomed] {
        let c = m.composition.unwrap();
        let names: Vec<_> = c
            .top_children
            .iter()
            .flatten()
            .map(|c| c.name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a", "b", "c"]);
        assert_eq!(remainder(&c, m.size), (1, 10));
        assert_eq!(
            c.top_children
                .iter()
                .flatten()
                .map(|c| c.size)
                .sum::<u128>()
                + remainder(&c, m.size).1,
            m.size
        );
    }
    assert!(
        files_in_folder(&root, 0)
            .iter()
            .find(|m| m.file_type == FileType::File)
            .unwrap()
            .composition
            .is_none()
    );
}

#[test]
fn display_cell_names_folder_suffix_counts_and_content_cap() {
    for name in [
        "this-is-a-very-long-name",
        "多字节的文件名和后缀",
        "e\u{301}e\u{301}e\u{301}e\u{301}",
    ] {
        let label = child_label(name, true, 0, 16);
        assert!(label.starts_with("#1 "));
        assert!(label.ends_with('/'));
        assert!(label.width() <= 16, "{label}");
    }
    let mut t = tile(100, 12, &[42, 40, 9, 9]);
    t.composition.as_mut().unwrap().top_children[0] = Some(ChildPreview {
        name: "长长长长长长长长长长长长长长".into(),
        size: 42,
        file_type: FileType::Folder,
    });
    let b = render(&t, CompositionPhase::Complete);
    let y = profile_y(&t);
    let x = t.x + 1 + (100 - 60) / 2;
    assert_eq!(b[(x, y + 1)].symbol(), "#");
    assert_eq!(b[(x + 59, y + 1)].symbol(), "%");
    assert_eq!(b[(x + 60, y + 1)].symbol(), " ");
    assert_eq!(counted(999, "", " more", 8), "999 more");
    assert_eq!(counted(10_000, "rest", "", 8), "rest>9k");
    let c = t.composition.as_mut().unwrap();
    c.direct_count = u64::MAX;
    for phase in [CompositionPhase::Complete, CompositionPhase::Partial] {
        for (w, h) in [(24, 6), (28, 10), (44, 10)] {
            t.width = w + 1;
            t.height = h + 1;
            let b = render(&t, phase);
            let y = if h == 6 { t.y + h } else { profile_y(&t) };
            let header = row(&b, y);
            assert!(header.contains('>') || header.contains(&u64::MAX.to_string()));
            assert!(header.trim().width() <= (w - 2) as usize);
            assert_eq!(
                header.trim().starts_with('~'),
                phase != CompositionPhase::Complete
            );
        }
    }
    let t = tile(24, 6, &[100]);
    let mut c = t.composition.unwrap();
    c.direct_count = 10_000;
    assert_eq!(
        header(&c, 100, CompositionPhase::Complete, false, 22),
        ">9 items, largest 100%"
    );
}

#[test]
fn composition_selection_and_track_styles_use_existing_theme_surfaces() {
    let mut t = tile(44, 10, &[42, 40, 9, 9]);
    t.composition.as_mut().unwrap().top_children[0]
        .as_mut()
        .unwrap()
        .name = "多字节的文件名和后缀".into();
    let x = t.x + 2;
    let y = profile_y(&t);
    for name in BUILTIN_SCHEMES {
        let theme = Theme::named(name).unwrap();
        for selected in [false, true] {
            let mut b = render(&t, CompositionPhase::Complete);
            draw_tile_text_on_grid(&mut b, &t, selected, &theme, CompositionPhase::Complete);
            let (ink, track) = theme.composition_styles(selected);
            assert_eq!(b[(x, y)].fg, ink.fg.unwrap_or(Color::Reset));
            assert_eq!(b[(x + 25, y + 1)].fg, track.fg.unwrap_or(Color::Reset));
            assert_eq!(b[(x, y)].modifier, Modifier::empty());
            if selected {
                for dy in 0..6 {
                    for dx in 0..42 {
                        assert_eq!(b[(x + dx, y + dy)].bg, theme.selected_folder.bg);
                    }
                }
            }
        }
    }
}

#[test]
fn deletion_refresh_rebuilds_rankings_and_leaves_no_stale_chart_cells() {
    let mut root = Folder::from(OsString::from("root"));
    for (i, size) in [42, 40, 9, 9].iter().enumerate() {
        root.add_file(PathBuf::from(format!("parent/child-{i}")), *size);
    }
    let mut board = Board::new(&root);
    let area = Rect::new(2, 2, 53, 13);
    board.change_area(&area);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 20));
    RectangleGrid::new(
        &board.tiles,
        None,
        None,
        &Theme::default(),
        CompositionPhase::Complete,
    )
    .render(area, &mut buffer);
    assert!(text(&buffer).contains("1 more"));
    root.delete_path(&["parent".into(), "child-1".into()]);
    board.change_files(&root);
    assert_eq!(board.tiles[0].size, 60);
    assert_eq!(board.tiles[0].composition.as_ref().unwrap().direct_count, 3);
    RectangleGrid::new(
        &board.tiles,
        None,
        None,
        &Theme::default(),
        CompositionPhase::Complete,
    )
    .render(area, &mut buffer);
    let refreshed = text(&buffer);
    assert!(refreshed.contains("70%"));
    assert!(!refreshed.contains("child-1"));
    assert!(!refreshed.contains("more"));
    // Switching to the incomplete/no-observation state must also erase old tracks.
    let mut t = board.tiles[0].clone();
    t.size = 0;
    t.composition = Some(FolderComposition {
        direct_count: 0,
        top_children: [None, None, None],
    });
    draw_tile_text_on_grid(
        &mut buffer,
        &t,
        false,
        &Theme::default(),
        CompositionPhase::Scanning,
    );
    assert!(text(&buffer).contains("~no items seen"));
    assert!(!text(&buffer).contains('['));
}

#[test]
fn file_and_below_threshold_tiles_ignore_composition_phase() {
    for (w, h, file) in [(23, 12, false), (52, 5, false), (52, 12, true)] {
        let mut t = tile(w, h, &[42, 40, 9, 9]);
        if file {
            t.file_type = FileType::File;
            t.descendants = None;
            t.composition = None;
        }
        assert_eq!(
            render(&t, CompositionPhase::Complete),
            render(&t, CompositionPhase::Scanning)
        );
    }
}
