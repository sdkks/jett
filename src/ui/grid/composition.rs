use ratatui::buffer::Buffer;
use ratatui::style::Style;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::state::tiles::{FileType, FolderComposition, Tile};
use crate::ui::theme::Theme;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionPhase {
    Scanning,
    Partial,
    Complete,
}

impl CompositionPhase {
    pub fn from_scan(loaded: bool, failed_to_read: u64) -> Self {
        match (loaded, failed_to_read) {
            (false, _) => Self::Scanning,
            (true, 0) => Self::Complete,
            (true, _) => Self::Partial,
        }
    }
}

fn dimensions(tile: &Tile) -> (u16, u16, bool) {
    let w = tile.width.saturating_sub(1);
    let h = tile.height.saturating_sub(1);
    (w, h, w >= 28 && h >= 10)
}

pub(super) fn identity_row(tile: &Tile) -> Option<u16> {
    let (w, h, profile) = dimensions(tile);
    (tile.file_type == FileType::Folder && tile.composition.is_some() && w >= 24 && h >= 6).then(
        || {
            if profile {
                1 + (h - 10) / 2
            } else {
                (h - 4) / 2
            }
        },
    )
}

// Exact quotient and remainder of size * scale / total, without overflowing u128.
// Scales are small (track cells or 100); metadata guarantees size <= total.
fn scaled_share(size: u128, total: u128, scale: u16) -> (u16, u128) {
    let mut quotient = (size / total) as u16 * scale;
    let part = size % total;
    let mut remainder = 0;
    for _ in 0..scale {
        if remainder >= total - part {
            remainder -= total - part;
            quotient += 1;
        } else {
            remainder += part;
        }
    }
    (quotient, remainder)
}

fn share(size: u128, total: u128) -> String {
    if size == 0 {
        return "0 B".into();
    }
    if size == total {
        return "100%".into();
    }
    let (whole, remainder) = scaled_share(size, total, 100);
    if whole == 0 {
        return "<1%".into();
    }
    let rounded = whole + u16::from(remainder >= total - remainder);
    if rounded == 100 {
        ">99%".into()
    } else {
        format!("{rounded}%")
    }
}

fn counted(count: u64, prefix: &str, suffix: &str, budget: usize) -> String {
    let exact = format!("{prefix}{count}{suffix}");
    if exact.width() <= budget {
        return exact;
    }
    // Explicit lower bounds, never a cut-off or silently rounded decimal count.
    for (unit, mark) in [
        (1_000, "k"),
        (1_000_000, "m"),
        (1_000_000_000, "b"),
        (1_000_000_000_000, "t"),
        (1_000_000_000_000_000, "q"),
        (1_000_000_000_000_000_000, "e"),
    ] {
        if count > unit {
            let compact = format!("{prefix}>{}{mark}{suffix}", (count - 1) / unit);
            if compact.width() <= budget {
                return compact;
            }
        }
    }
    // At the narrowest summary tier even a unit suffix may not fit beside 100%.
    // Keep the exact wording and an explicit (coarser) bound rather than cutting digits.
    let digits = budget - prefix.width() - suffix.width() - 1;
    let bound = 10_u64.pow(digits as u32) - 1;
    format!("{prefix}>{bound}{suffix}")
}

fn header(
    composition: &FolderComposition,
    total: u128,
    phase: CompositionPhase,
    profile: bool,
    budget: usize,
) -> String {
    let count = composition.direct_count;
    let incomplete = phase != CompositionPhase::Complete;
    if count == 0 {
        return if incomplete {
            "~no items seen"
        } else {
            "empty (0 items)"
        }
        .into();
    }
    let separator = if profile { ";" } else { "," };
    let ending = if total == 0 {
        if incomplete {
            "0 B so far".into()
        } else {
            "all 0 B".into()
        }
    } else if profile {
        "% of folder".into()
    } else {
        let largest = composition.top_children[0]
            .as_ref()
            .expect("observed children have a largest item");
        format!("largest {}", share(largest.size, total))
    };
    let suffix = format!(
        "{}{separator} {ending}",
        if incomplete { " seen" } else { " items" }
    );
    counted(count, if incomplete { "~" } else { "" }, &suffix, budget)
}

fn take_cells(chars: impl Iterator<Item = char>, budget: usize) -> String {
    let mut width = 0;
    chars
        .take_while(|ch| {
            width += ch.width().unwrap_or(0);
            width <= budget
        })
        .collect()
}

fn child_label(name: &str, folder: bool, rank: usize, budget: usize) -> String {
    let prefix = format!("#{} ", rank + 1);
    let suffix = if folder { "/" } else { "" };
    let available = budget - prefix.width() - suffix.width();
    let name = if name.width() > available {
        let left = (available - 3) / 2;
        let right = available - 3 - left;
        let start = take_cells(name.chars(), left);
        let end: String = take_cells(name.chars().rev(), right)
            .chars()
            .rev()
            .collect();
        format!("{start}...{end}")
    } else {
        name.into()
    };
    format!("{prefix}{name}{suffix}")
}

fn remainder(composition: &FolderComposition, total: u128) -> (u64, u128) {
    let children = composition.top_children.iter().flatten();
    let (count, size) = children.fold((0, 0), |(count, size), child| {
        (count + 1, size + child.size)
    });
    (composition.direct_count - count, total - size)
}

struct ProfileRow<'a> {
    label: &'a str,
    size: u128,
    mark: char,
}

fn draw_row(
    buf: &mut Buffer,
    position: (u16, u16),
    widths: (u16, u16),
    row: ProfileRow<'_>,
    total: u128,
    styles: (Style, Style),
) {
    let (x, y) = position;
    let (label_width, track) = widths;
    let (ink, muted) = styles;
    let padding = " ".repeat(label_width as usize - row.label.width());
    buf.set_string(x, y, format!("{}{padding} [", row.label), ink);
    let start = x + label_width + 2;
    let filled = scaled_share(row.size, total, track).0;
    buf.set_string(start, y, row.mark.to_string().repeat(filled as usize), ink);
    buf.set_string(
        start + filled,
        y,
        ".".repeat((track - filled) as usize),
        muted,
    );
    buf.set_string(
        start + track,
        y,
        format!("] {:>4}", share(row.size, total)),
        ink,
    );
}

pub(super) fn draw_composition(
    buf: &mut Buffer,
    tile: &Tile,
    selected: bool,
    theme: &Theme,
    phase: CompositionPhase,
) {
    let composition = tile
        .composition
        .as_ref()
        .expect("eligible folder has composition");
    let (w, h, profile) = dimensions(tile);
    let content = (w - 2).min(60);
    let x = tile.x + 1 + (w - content) / 2;
    let y = tile.y + 1 + if profile { h - 6 } else { h - 1 };
    let styles = theme.composition_styles(selected);
    buf.set_string(
        x,
        y,
        header(composition, tile.size, phase, profile, content as usize),
        styles.0,
    );
    if !profile || tile.size == 0 || composition.direct_count == 0 {
        return;
    }
    let label_width = if w >= 44 { 16 } else { 8 };
    let track = content - label_width - 8;
    for (rank, child) in composition.top_children.iter().enumerate() {
        if let Some(child) = child {
            let label = if w >= 44 {
                child_label(
                    &child.name.to_string_lossy(),
                    child.file_type == FileType::Folder,
                    rank,
                    label_width as usize,
                )
            } else {
                ["largest", "2nd", "3rd"][rank].into()
            };
            draw_row(
                buf,
                (x, y + 1 + rank as u16),
                (label_width, track),
                ProfileRow {
                    label: &label,
                    size: child.size,
                    mark: '#',
                },
                tile.size,
                styles,
            );
        }
    }
    let (count, size) = remainder(composition, tile.size);
    if count > 0 {
        let exact = format!("{count} more");
        let label = if exact.width() <= label_width as usize {
            exact
        } else {
            counted(count, "rest", "", label_width as usize)
        };
        draw_row(
            buf,
            (x, y + 4),
            (label_width, track),
            ProfileRow {
                label: &label,
                size,
                mark: '=',
            },
            tile.size,
            styles,
        );
    }
}
