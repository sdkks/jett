use anyhow::{Result, bail};
use ratatui::style::Color;
use toml::Value;

use crate::ui::theme::Theme;

pub(super) fn apply(theme: &mut Theme, role: &str, value: &Value) -> Result<()> {
    let target = match role {
        "foreground" => &mut theme.foreground,
        "title" => &mut theme.title,
        "title_separator" => &mut theme.title_separator,
        "accent" => &mut theme.accent,
        "success" => &mut theme.success,
        "error" => &mut theme.error,
        "warning" => &mut theme.warning,
        "path_error_fg" => &mut theme.path_error.fg,
        "path_error_bg" => &mut theme.path_error.bg,
        "freed_flash_fg" => &mut theme.freed_flash.fg,
        "freed_flash_bg" => &mut theme.freed_flash.bg,
        "modal_surface" => &mut theme.modal_surface,
        "modal_text" => &mut theme.modal_text,
        "tile_text" => &mut theme.tile_text,
        "tile_accent" => &mut theme.tile_accent,
        "selected_file_fg" => &mut theme.selected_file.fg,
        "selected_file_bg" => &mut theme.selected_file.bg,
        "selected_folder_bg" => &mut theme.selected_folder.bg,
        "selected_folder_line1_fg" => &mut theme.selected_folder.line1_fg,
        "selected_folder_line2_fg" => &mut theme.selected_folder.line2_fg,
        "legend_chip_fg" => &mut theme.legend_chip.fg,
        "legend_chip_bg" => &mut theme.legend_chip.bg,
        "empty_surface_fg" => &mut theme.empty_surface.fg,
        "empty_surface_bg" => &mut theme.empty_surface.bg,
        "tile_fill" => &mut theme.tile_fill,
        "tile_border" => &mut theme.tile_border,
        "tile_composition" => &mut theme.tile_composition,
        "tile_composition_track" => &mut theme.tile_composition_track,
        _ => bail!("unknown color role '{role}'"),
    };
    *target = parse(value)?;
    Ok(())
}

fn parse(value: &Value) -> Result<Color> {
    if let Value::Table(table) = value
        && table.len() == 1
        && let Some(Value::Array(channels)) = table.get("rgb")
        && channels.len() == 3
    {
        let channels: Option<Vec<u8>> = channels
            .iter()
            .map(|channel| {
                channel
                    .as_integer()
                    .and_then(|channel| u8::try_from(channel).ok())
            })
            .collect();
        if let Some(channels) = channels {
            return Ok(Color::Rgb(channels[0], channels[1], channels[2]));
        }
    }
    if let Value::String(text) = value {
        if let Some(hex) = text.strip_prefix('#') {
            if matches!(hex.len(), 3 | 6 | 8) && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let hex = if hex.len() == 3 {
                    hex.chars()
                        .flat_map(|digit| [digit, digit])
                        .collect::<String>()
                } else {
                    hex[..6].to_owned()
                };
                let rgb = u32::from_str_radix(&hex, 16).expect("hex digits validated above");
                return Ok(Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
            }
        } else {
            let name: String = text
                .chars()
                .filter(|char| !matches!(char, ' ' | '-' | '_'))
                .flat_map(char::to_lowercase)
                .collect();
            let color = match name.as_str() {
                "reset" | "none" => Color::Reset,
                "black" => Color::Black,
                "red" => Color::Red,
                "green" => Color::Green,
                "yellow" => Color::Yellow,
                "blue" => Color::Blue,
                "magenta" => Color::Magenta,
                "cyan" => Color::Cyan,
                "gray" | "grey" => Color::Gray,
                "darkgray" | "darkgrey" => Color::DarkGray,
                "lightred" => Color::LightRed,
                "lightgreen" => Color::LightGreen,
                "lightyellow" => Color::LightYellow,
                "lightblue" => Color::LightBlue,
                "lightmagenta" => Color::LightMagenta,
                "lightcyan" => Color::LightCyan,
                "white" => Color::White,
                _ => bail!(
                    "invalid color '{text}': expected a named color, #RGB, #RRGGBB, #RRGGBBAA, or {{ rgb = [r, g, b] }}"
                ),
            };
            return Ok(color);
        }
    }
    bail!(
        "invalid color: expected a named color, #RGB, #RRGGBB, #RRGGBBAA, or {{ rgb = [r, g, b] }} with integer channels from 0 to 255"
    )
}
