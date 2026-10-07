use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::style::{Modifier, Style};
use ::ratatui::widgets::Widget;
use ::std::path::{Path, PathBuf};

use crate::state::tiles::{FileType, Tile};
use crate::ui::format::{DisplaySize, truncate_middle};
use crate::ui::theme::Theme;

fn render_currently_selected(
    buf: &mut Buffer,
    currently_selected: &Tile,
    max_len: u16,
    y: u16,
    theme: &Theme,
) {
    let file_name = currently_selected.name.to_string_lossy();
    let size = DisplaySize(currently_selected.size as f64);
    let descendants = currently_selected.descendants;
    let (style, lines) = match currently_selected.file_type {
        FileType::File => (
            theme.text_style().add_modifier(Modifier::BOLD),
            vec![
                format!("SELECTED: {} ({})", file_name, size),
                format!("SELECTED: {}", file_name),
                format!("{}", file_name),
            ],
        ),
        FileType::Folder => (
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
            vec![
                format!(
                    "SELECTED: {} ({}, {} files)",
                    file_name,
                    size,
                    descendants.expect("a folder should have descendants")
                ),
                format!("SELECTED: {} ({})", file_name, size),
                format!("SELECTED: {}", file_name),
                format!("{}", file_name),
            ],
        ),
    };
    for line in lines {
        if (line.chars().count() as u16) < max_len {
            buf.set_string(1, y, line, style);
            break;
        }
    }
}

fn render_last_read_path(
    buf: &mut Buffer,
    last_read_path: &Path,
    max_len: u16,
    y: u16,
    theme: &Theme,
) {
    let last_read_path = last_read_path.to_string_lossy();
    if (last_read_path.chars().count() as u16) < max_len {
        buf.set_string(1, y, last_read_path, theme.text_style());
    } else {
        buf.set_string(
            1,
            y,
            truncate_middle(&last_read_path, max_len),
            theme.text_style(),
        );
    }
}

fn render_controls_legend(
    buf: &mut Buffer,
    hide_delete: bool,
    show_themes: bool,
    max_len: u16,
    y: u16,
    theme: &Theme,
) {
    let (mut long_controls_line, mut short_controls_line) = if hide_delete {
        (
            String::from(
                "<arrows> - move, <ENTER> - enter folder, <ESC> - parent, <o> - open folder, <y> - copy path, <+/-/0> - zoom, <q> - quit",
            ),
            String::from("←↓↑→/<ENTER>/<ESC>: navigate"),
        )
    } else {
        (
            String::from(
                "<arrows> - move, <ENTER> - enter folder, <ESC> - parent, <BACKSPACE> - delete, <o> - open folder, <y> - copy path, <+/-/0> - zoom, <q> - quit",
            ),
            String::from("←↓↑→/<ENTER>/<ESC>: navigate, <BACKSPACE>: del"),
        )
    };
    if max_len >= short_controls_line.chars().count() as u16 + 22 {
        short_controls_line.push_str(", <o>: open, <y>: copy");
    }
    if show_themes {
        long_controls_line = long_controls_line.replace("<q> - quit", "<t> - themes, <q> - quit");
        if max_len >= short_controls_line.chars().count() as u16 + 13 {
            short_controls_line.push_str(", <t>: themes");
        }
    }
    let mut compact_controls_line = if hide_delete {
        String::from("←↓↑→: navigate")
    } else {
        String::from("←↓↑→: navigate, ⌫: del")
    };
    if show_themes && max_len >= compact_controls_line.chars().count() as u16 + 13 {
        compact_controls_line.push_str(", <t>: themes");
    }
    let too_small_line = "(...)";
    if max_len >= long_controls_line.chars().count() as u16 {
        buf.set_string(
            1,
            y,
            long_controls_line,
            theme.text_style().add_modifier(Modifier::BOLD),
        );
    } else if max_len >= short_controls_line.chars().count() as u16 {
        buf.set_string(
            1,
            y,
            short_controls_line,
            theme.text_style().add_modifier(Modifier::BOLD),
        );
    } else if max_len >= compact_controls_line.chars().count() as u16 {
        buf.set_string(
            1,
            y,
            compact_controls_line,
            theme.text_style().add_modifier(Modifier::BOLD),
        );
    } else {
        buf.set_string(
            1,
            y,
            too_small_line,
            theme.text_style().add_modifier(Modifier::BOLD),
        );
    }
}

fn render_small_files_legend(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    small_files_legend: &str,
    theme: &Theme,
) {
    buf.set_string(
        x,
        y,
        small_files_legend,
        theme.legend_text_style().remove_modifier(Modifier::all()),
    );
    let small_files_legend_character = &mut buf[(x + 1, y)];
    small_files_legend_character.set_style(theme.legend_chip.style());
}

pub(crate) fn mode_chip_text(dry_run: bool, delete_confirmation_disabled: bool) -> &'static str {
    if dry_run {
        "DRY-RUN ON"
    } else if delete_confirmation_disabled {
        "DRY-RUN OFF + CONFIRM OFF"
    } else {
        "DRY-RUN OFF"
    }
}

pub(crate) fn render_mode_chip(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    dry_run: bool,
    delete_confirmation_disabled: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let text = mode_chip_text(dry_run, delete_confirmation_disabled);
    let color = if dry_run {
        theme.success
    } else if delete_confirmation_disabled {
        theme.error
    } else {
        theme.warning
    };
    let width = (text.len() as u16).min(area.width.saturating_sub(1));
    buf.set_stringn(
        area.right() - width - 1,
        area.bottom() - 1,
        text,
        usize::from(width),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    );
}

pub(crate) struct ModeChip<'a> {
    pub theme: &'a Theme,
    pub effects: &'a crate::state::UiEffects,
}

impl Widget for ModeChip<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        render_mode_chip(
            area,
            buf,
            self.theme,
            self.effects.dry_run,
            self.effects.delete_confirmation_disabled,
        );
    }
}

pub struct BottomLine<'a> {
    theme: &'a Theme,
    hide_delete: bool,
    show_themes: bool,
    dry_run: bool,
    delete_confirmation_disabled: bool,
    hide_small_files_legend: bool,
    currently_selected: Option<&'a Tile>,
    last_read_path: Option<&'a PathBuf>,
    notice: Option<&'a str>,
}

impl<'a> BottomLine<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            hide_delete: false,
            show_themes: false,
            dry_run: false,
            delete_confirmation_disabled: false,
            hide_small_files_legend: false,
            currently_selected: None,
            last_read_path: None,
            notice: None,
        }
    }
    pub fn deletion_mode(mut self, dry_run: bool, delete_confirmation_disabled: bool) -> Self {
        self.dry_run = dry_run;
        self.delete_confirmation_disabled = delete_confirmation_disabled;
        self
    }
    pub fn notice(mut self, notice: Option<&'a str>) -> Self {
        self.notice = notice;
        self
    }
    pub fn show_themes(mut self) -> Self {
        self.show_themes = true;
        self
    }
    pub fn hide_delete(mut self) -> Self {
        self.hide_delete = true;
        self
    }
    pub fn hide_small_files_legend(mut self, should_hide_small_files_legend: bool) -> Self {
        self.hide_small_files_legend = should_hide_small_files_legend;
        self
    }
    pub fn currently_selected(mut self, currently_selected: Option<&'a Tile>) -> Self {
        self.currently_selected = currently_selected;
        self
    }
    pub fn last_read_path(mut self, last_read_path: Option<&'a PathBuf>) -> Self {
        self.last_read_path = last_read_path;
        self
    }
}

impl<'a> Widget for BottomLine<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let small_files_legend = "(x = Small files)";
        let small_files_len = if self.hide_small_files_legend || self.notice.is_some() {
            0
        } else {
            small_files_legend.chars().count() as u16
        };
        let max_status_len = area.width - small_files_len - 1;
        let chip_len = mode_chip_text(self.dry_run, self.delete_confirmation_disabled).len() as u16;
        let max_controls_len = area.width.saturating_sub(chip_len + 3);
        let status_line_y = area.y + area.height - 2;
        let controls_line_y = status_line_y + 1;
        if let Some(notice) = self.notice {
            let notice: String = notice
                .chars()
                .map(|char| if char.is_control() { ' ' } else { char })
                .collect();
            buf.set_stringn(
                1,
                status_line_y,
                notice,
                usize::from(max_status_len),
                Style::default()
                    .fg(self.theme.warning)
                    .add_modifier(Modifier::BOLD),
            );
        } else if let Some(currently_selected) = self.currently_selected {
            render_currently_selected(
                buf,
                currently_selected,
                max_status_len,
                status_line_y,
                self.theme,
            );
        } else if let Some(last_read_path) = self.last_read_path {
            render_last_read_path(
                buf,
                last_read_path,
                max_status_len,
                status_line_y,
                self.theme,
            );
        }

        if !self.hide_small_files_legend && self.notice.is_none() {
            render_small_files_legend(
                buf,
                area.width - small_files_len - 1,
                status_line_y,
                small_files_legend,
                self.theme,
            );
        }

        render_controls_legend(
            buf,
            self.hide_delete,
            self.show_themes,
            max_controls_len,
            controls_line_y,
            self.theme,
        );
    }
}
