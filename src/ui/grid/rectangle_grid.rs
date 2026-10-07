use ::ratatui::buffer::Buffer;
use ::ratatui::layout::Rect;
use ::ratatui::widgets::Widget;

use crate::state::tiles::Tile;
use crate::ui::grid::{CompositionPhase, draw_rect_on_grid, draw_tile_text_on_grid};
use crate::ui::theme::Theme;

fn draw_small_files_rect_on_grid(buf: &mut Buffer, rect: Rect, theme: &Theme) {
    for x in rect.x + 1..(rect.x + rect.width) {
        for y in rect.y + 1..(rect.y + rect.height) {
            let buf = &mut buf[(x, y)];
            buf.set_symbol("x");
            buf.set_style(theme.legend_chip.style());
        }
    }
    draw_rect_on_grid(buf, (rect.x, rect.y), (rect.width, rect.height));
}

fn draw_empty_folder(buf: &mut Buffer, area: Rect, theme: &Theme) {
    for x in area.x + 1..area.x + area.width {
        for y in area.y + 1..area.y + area.height {
            let buf = &mut buf[(x, y)];
            buf.set_symbol("█");
            buf.set_style(theme.empty_surface.style());
        }
    }
    let empty_folder_line = "Folder is empty";
    let text_length = empty_folder_line.len();
    let text_style = theme.empty_text_style();
    let text_start_position =
        ((area.width - text_length as u16) as f64 / 2.0).ceil() as u16 + area.x;
    buf.set_string(
        text_start_position,
        (area.height / 2) + area.y - 1,
        empty_folder_line,
        text_style,
    );
    draw_rect_on_grid(buf, (area.x, area.y), (area.width, area.height));
}

#[derive(Clone)]
pub struct RectangleGrid<'a> {
    theme: &'a Theme,
    composition_phase: CompositionPhase,
    rectangles: &'a [Tile],
    small_files_coordinates: Option<(u16, u16)>,
    selected_rect_index: Option<usize>,
}

impl<'a> RectangleGrid<'a> {
    pub fn new(
        rectangles: &'a [Tile],
        small_files_coordinates: Option<(u16, u16)>,
        selected_rect_index: Option<usize>,
        theme: &'a Theme,
        composition_phase: CompositionPhase,
    ) -> Self {
        RectangleGrid {
            theme,
            composition_phase,
            rectangles,
            small_files_coordinates,
            selected_rect_index,
        }
    }
}

impl<'a> Widget for RectangleGrid<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.rectangles.is_empty() {
            draw_empty_folder(buf, area, self.theme);
        } else {
            for (index, tile) in self.rectangles.iter().enumerate() {
                let selected = if let Some(selected_rect_index) = self.selected_rect_index {
                    index == selected_rect_index
                } else {
                    false
                };
                draw_tile_text_on_grid(buf, tile, selected, self.theme, self.composition_phase);
                draw_rect_on_grid(buf, (tile.x, tile.y), (tile.width, tile.height));
            }
        }
        if let Some(coords) = self.small_files_coordinates {
            let (x, y) = coords;
            let width = (area.x + area.width) - x;
            let height = (area.y + area.height) - y;
            let small_files_rect = Rect {
                x,
                y,
                width,
                height,
            };
            draw_small_files_rect_on_grid(buf, small_files_rect, self.theme);
        }
    }
}
