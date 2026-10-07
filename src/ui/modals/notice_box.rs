use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::style::Modifier;
use ::ratatui::widgets::Widget;

use crate::ui::format::truncate_middle;
use crate::ui::grid::draw_filled_rect;
use crate::ui::theme::Theme;

// Transient feedback overlay for copy/open actions. It dismisses itself
// after a couple of seconds (Event::FlashNotice drives the timeout) and
// any keypress dismisses it early (handle_keypress_transient_notice).
pub struct NoticeBox<'a> {
    theme: &'a Theme,
    message: &'a str,
}

impl<'a> NoticeBox<'a> {
    pub fn new(message: &'a str, theme: &'a Theme) -> Self {
        Self { theme, message }
    }
}

impl Widget for NoticeBox<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // The app switches to ScreenTooSmall below these bounds, so this
        // guard is defensive only; never panic while drawing feedback.
        if area.width < 50 || area.height < 15 {
            return;
        }
        let width = (area.width / 2).min(150);
        let height = 6;
        let x = ((area.x + area.width) / 2) - width / 2;
        let y = ((area.y + area.height) / 2) - height / 2;
        let message_rect = Rect {
            x,
            y,
            width,
            height,
        };
        let fill_style = self
            .theme
            .modal_style(self.theme.success)
            .add_modifier(Modifier::BOLD);
        let text_max_length = message_rect.width - 4;
        // Paths keep both ends visible when the line must shrink.
        let text = truncate_middle(self.message, text_max_length);
        let text_start = ((message_rect.width - text.chars().count() as u16) as f64 / 2.0).ceil()
            as u16
            + message_rect.x;
        draw_filled_rect(buf, fill_style, &message_rect);
        buf.set_string(
            text_start,
            message_rect.y + message_rect.height / 2,
            text,
            fill_style,
        );
    }
}
