use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::style::Modifier;
use ::ratatui::widgets::Widget;

use crate::state::FileToDelete;
use crate::state::tiles::FileType;
use crate::ui::format::truncate_middle;
use crate::ui::grid::draw_filled_rect;
use crate::ui::theme::Theme;

fn truncated_file_name_line(file_to_delete: &FileToDelete, max_len: u16) -> String {
    let full_path = file_to_delete
        .full_path()
        .into_os_string()
        .into_string()
        .expect("could not convert os string to string");
    let file_name = file_to_delete
        .path_to_file
        .last()
        .expect("could not find file to delete")
        .to_string_lossy();
    #[cfg(test)]
    let full_path = str::replace(&full_path, "\\", "/");
    if max_len > full_path.len() as u16 {
        full_path
    } else {
        truncate_middle(&file_name, max_len)
    }
}

fn render_deletion_prompt(
    buf: &mut Buffer,
    message_rect: &Rect,
    file_to_delete: &FileToDelete,
    theme: &Theme,
) {
    let max_text_len = message_rect.width - 4;
    let file_name_line = truncated_file_name_line(file_to_delete, max_text_len);
    let text_style = theme.modal_style(theme.error).add_modifier(Modifier::BOLD);
    let question_line = match file_to_delete.file_type {
        FileType::File => {
            if max_text_len >= 17 {
                "Delete this file?".to_string()
            } else if max_text_len >= 3 {
                "Delete?".to_string()
            } else {
                unreachable!("should not render if terminal is so small");
            }
        }
        FileType::Folder => {
            let children = file_to_delete
                .num_descendants
                .expect("folder should have descendants");
            let full_line = format!("Delete folder with {} children?", children);
            let short_line = "Delete folder?".to_string();
            if max_text_len >= full_line.len() as u16 {
                full_line
            } else if max_text_len >= short_line.len() as u16 {
                short_line
            } else {
                unreachable!("should not render if terminal is so small");
            }
        }
    };
    let y_n_line = "(y/n)";
    let question_line_start_position =
        ((message_rect.width - question_line.len() as u16) as f64 / 2.0).ceil() as u16
            + message_rect.x;
    let file_name_line_start_position =
        ((message_rect.width - file_name_line.len() as u16) as f64 / 2.0).ceil() as u16
            + message_rect.x;
    let y_n_line_start_position =
        ((message_rect.width - y_n_line.len() as u16) as f64 / 2.0).ceil() as u16 + message_rect.x;
    buf.set_string(
        question_line_start_position,
        message_rect.y + message_rect.height / 2 - 3,
        question_line,
        text_style,
    );
    buf.set_string(
        file_name_line_start_position,
        message_rect.y + message_rect.height / 2,
        file_name_line,
        text_style,
    );
    buf.set_string(
        y_n_line_start_position,
        message_rect.y + message_rect.height / 2 + 3,
        y_n_line,
        text_style,
    );
}

fn render_deletion_in_progress(
    buf: &mut Buffer,
    message_rect: &Rect,
    file_to_delete: &FileToDelete,
    theme: &Theme,
    dry_run: bool,
) {
    let max_text_len = message_rect.width - 4;
    let file_name_line = truncated_file_name_line(file_to_delete, max_text_len);
    let deleting_line = if dry_run {
        "Planning deletion"
    } else {
        "Deleting"
    };
    let text_style = theme.modal_style(theme.error).add_modifier(Modifier::BOLD);
    let deleting_line_start_position =
        ((message_rect.width - deleting_line.len() as u16) as f64 / 2.0).ceil() as u16
            + message_rect.x;
    let file_line_start_position = ((message_rect.width - file_name_line.len() as u16) as f64 / 2.0)
        .ceil() as u16
        + message_rect.x;
    buf.set_string(
        deleting_line_start_position,
        message_rect.y + message_rect.height / 2 - 1,
        deleting_line,
        text_style,
    );
    buf.set_string(
        file_line_start_position,
        message_rect.y + message_rect.height / 2 + 1,
        file_name_line,
        text_style,
    );
}

pub struct MessageBox<'a> {
    theme: &'a Theme,
    file_to_delete: &'a FileToDelete,
    deletion_in_progress: bool,
    dry_run: bool,
}

impl<'a> MessageBox<'a> {
    pub fn new(
        file_to_delete: &'a FileToDelete,
        deletion_in_progress: bool,
        theme: &'a Theme,
    ) -> Self {
        Self {
            theme,
            file_to_delete,
            deletion_in_progress,
            dry_run: false,
        }
    }
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }
}

impl<'a> Widget for MessageBox<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (width, height) = if area.width > 150 {
            (150, 10)
        } else if area.width >= 50 {
            (area.width / 2, 10)
        } else {
            unreachable!("app should not be rendered if window is so small")
        };

        // position self in the middle of the rect
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
            .modal_style(self.theme.error)
            .add_modifier(Modifier::BOLD);

        draw_filled_rect(buf, fill_style, &message_rect);
        if self.deletion_in_progress {
            render_deletion_in_progress(
                buf,
                &message_rect,
                self.file_to_delete,
                self.theme,
                self.dry_run,
            );
        } else {
            render_deletion_prompt(buf, &message_rect, self.file_to_delete, self.theme);
        }
        if self.dry_run {
            let text = if message_rect.width >= 34 {
                "DRY-RUN: nothing will be deleted"
            } else {
                "DRY-RUN: no deletion"
            };
            buf.set_string(
                message_rect.x + (message_rect.width - text.len() as u16) / 2,
                message_rect.y + message_rect.height / 2 - 2,
                text,
                self.theme
                    .modal_style(self.theme.success)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }
}
