use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::{Clear, Widget};

use crate::state::theme_selector::ThemeSelector;
use crate::ui::theme::Theme;

pub struct ThemeSelectorView<'a> {
    pub selector: &'a ThemeSelector,
    pub theme: &'a Theme,
    pub effects: &'a crate::state::UiEffects,
}

impl Widget for ThemeSelectorView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The selector owns the screen's input; the real treemap remains its live preview.
        let panel = Rect::new(
            area.x,
            area.y + 1,
            (area.width / 2).min(44),
            area.height - 3,
        );
        Clear.render(panel, buf);
        let surface = self.theme.modal_style(self.theme.modal_text);
        buf.set_style(panel, surface);
        let width = usize::from(panel.width.saturating_sub(2));
        buf.set_stringn(
            panel.x + 1,
            panel.y,
            "Themes (live preview)",
            width,
            surface.add_modifier(Modifier::BOLD),
        );
        let rows = usize::from(panel.height.saturating_sub(2));
        let start = self
            .selector
            .highlighted
            .saturating_sub(rows.saturating_sub(1));
        if self.selector.matches.is_empty() {
            buf.set_stringn(
                panel.x + 1,
                panel.y + 2,
                "No matching themes",
                width,
                surface,
            );
        } else {
            for (row, index) in self
                .selector
                .matches
                .iter()
                .skip(start)
                .take(rows)
                .enumerate()
            {
                let highlighted = start + row == self.selector.highlighted;
                let prefix = if highlighted { "> " } else { "  " };
                let name = &self.selector.choices[*index].name;
                let style = if highlighted {
                    self.theme
                        .selected_file
                        .style()
                        .add_modifier(Modifier::BOLD)
                } else {
                    surface
                };
                buf.set_stringn(
                    panel.x + 1,
                    panel.y + 2 + row as u16,
                    format!("{prefix}{name}"),
                    width,
                    style,
                );
            }
        }
        let bottom = Rect::new(area.x, area.bottom() - 2, area.width, 2);
        Clear.render(bottom, buf);
        buf.set_style(bottom, surface);
        let filter = self
            .selector
            .visible_filter(usize::from(area.width.saturating_sub(11)));
        buf.set_stringn(
            area.x + 1,
            bottom.y,
            format!("Filter: {filter}_"),
            usize::from(area.width - 2),
            surface,
        );
        let chip_width = crate::ui::bottom_line::mode_chip_text(
            self.effects.dry_run,
            self.effects.delete_confirmation_disabled,
        )
        .len() as u16;
        let help_width = area.width.saturating_sub(chip_width + 3);
        let long_help = "Type to filter, <Up/Down> - preview, <Enter> - save, <Esc> - cancel, <q> - cancel if filter empty";
        let help = if usize::from(help_width) >= long_help.chars().count() {
            long_help
        } else if help_width >= 39 {
            "Up/Down: preview, Enter: save, Esc: cancel"
        } else if help_width >= 28 {
            "↑↓ preview, ↵ save, Esc cancel"
        } else {
            "↑↓, ↵ save, Esc cancel"
        };
        buf.set_stringn(
            area.x + 1,
            bottom.y + 1,
            help,
            usize::from(help_width),
            surface.add_modifier(Modifier::BOLD),
        );
    }
}
