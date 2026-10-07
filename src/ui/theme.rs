//! Jett's semantic colors. Built-in palette seeds follow OPI's builtins.rs;
//! interaction state and text emphasis belong to the widgets, not the palette.

use ratatui::style::{Color, Style};

pub const BUILTIN_SCHEMES: &[&str] = &[
    "default",
    "dark",
    "light",
    "solarized-dark",
    "solarized-light",
    "monokai",
    "dracula",
    "gruvbox-dark",
    "gruvbox-light",
    "nord",
    "catppuccin-mocha",
    "catppuccin-macchiato",
    "catppuccin-frappe",
    "catppuccin-latte",
    "tokyo-night",
    "tokyo-night-storm",
    "tokyo-night-day",
    "rose-pine",
    "rose-pine-moon",
    "rose-pine-dawn",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorPair {
    pub fg: Color,
    pub bg: Color,
}

impl ColorPair {
    pub fn style(self) -> Style {
        Style::default().fg(self.fg).bg(self.bg)
    }
    pub fn fill_style(self) -> Style {
        Style::default().fg(self.bg).bg(self.bg)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FolderSelection {
    pub bg: Color,
    pub line1_fg: Color,
    pub line2_fg: Color,
}

impl FolderSelection {
    pub fn first_line_style(self) -> Style {
        Style::default().fg(self.line1_fg).bg(self.bg)
    }
    pub fn second_line_style(self) -> Style {
        Style::default().fg(self.line2_fg).bg(self.bg)
    }
    pub fn fill_style(self) -> Style {
        Style::default().fg(self.bg).bg(self.bg)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub foreground: Color,
    pub title: Color,
    // The legacy pipe is white rather than the ordinary reset foreground.
    pub title_separator: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub path_error: ColorPair,
    pub freed_flash: ColorPair,
    pub modal_surface: Color,
    pub modal_text: Color,
    pub tile_text: Color,
    pub tile_accent: Color,
    pub selected_file: ColorPair,
    pub selected_folder: FolderSelection,
    pub legend_chip: ColorPair,
    pub empty_surface: ColorPair,
    // Reserved for tile composition; ordinary tiles currently inherit the terminal surface.
    pub tile_fill: Color,
    pub tile_border: Color,
    pub tile_composition: Color,
    pub tile_composition_track: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: ThemeMode::Dark,
            foreground: Color::Reset,
            title: Color::Yellow,
            title_separator: Color::White,
            accent: Color::Blue,
            success: Color::Green,
            error: Color::Red,
            warning: Color::Yellow,
            path_error: ColorPair {
                fg: Color::White,
                bg: Color::Red,
            },
            freed_flash: ColorPair {
                fg: Color::Black,
                bg: Color::Yellow,
            },
            modal_surface: Color::Black,
            modal_text: Color::White,
            tile_text: Color::Reset,
            tile_accent: Color::Blue,
            selected_file: ColorPair {
                fg: Color::Magenta,
                bg: Color::Gray,
            },
            selected_folder: FolderSelection {
                bg: Color::Blue,
                line1_fg: Color::White,
                line2_fg: Color::Black,
            },
            legend_chip: ColorPair {
                fg: Color::Black,
                bg: Color::White,
            },
            empty_surface: ColorPair {
                fg: Color::Black,
                bg: Color::White,
            },
            tile_fill: Color::Reset,
            tile_border: Color::Reset,
            tile_composition: Color::Reset,
            tile_composition_track: Color::Reset,
        }
    }
}

impl Theme {
    /// Exact, case-sensitive built-in lookup. Unknown names never silently fall back.
    pub fn named(name: &str) -> Option<Self> {
        use ThemeMode::{Dark, Light};
        if name == "default" {
            return Some(Self::default());
        }
        // foreground, base, muted, accent, selection, success, warning, error.
        // Named ANSI seeds in OPI's dark scheme become explicit RGB here so the
        // text-on-fill target is measurable. Default alone retains legacy ANSI values.
        // Error/warning seeds below are lightened/darkened where OPI's
        // originals fall below 4.5:1 on the modal base, preserving their hue.
        let (mode, colors) = match name {
            "dark" => (
                Dark,
                [
                    0xe0e0e0, 0x000000, 0x808080, 0x00ffff, 0x264f78, 0x008000, 0xffff00, 0xff0000,
                ],
            ),
            "light" => (
                Light,
                [
                    0x202124, 0xffffff, 0x6b7280, 0x0b57d0, 0xdbeafe, 0x137333, 0xb06000, 0xb3261e,
                ],
            ),
            "solarized-dark" => (
                Dark,
                [
                    0x839496, 0x002b36, 0x586e75, 0x268bd2, 0x073642, 0x859900, 0xb58900,
                    0xe56765, // error: #dc322f → lighter
                ],
            ),
            "solarized-light" => (
                Light,
                [
                    // OPI's secondary foreground is stronger than #657b83 on this base.
                    0x586e75, 0xfdf6e3, 0x93a1a1, 0x268bd2, 0xeee8d5, 0x859900, 0x8b6900,
                    0xd3302d, // warning/error darkened
                ],
            ),
            "monokai" => (
                Dark,
                [
                    0xf8f8f2, 0x272822, 0x75715e, 0x66d9ef, 0x49483e, 0xa6e22e, 0xe6db74,
                    0xfa4f8d, // error: #f92672 → lighter
                ],
            ),
            "dracula" => (
                Dark,
                [
                    0xf8f8f2, 0x282a36, 0x6272a4, 0xbd93f9, 0x44475a, 0x50fa7b, 0xf1fa8c, 0xff5555,
                ],
            ),
            "gruvbox-dark" => (
                Dark,
                [
                    0xebdbb2, 0x282828, 0x928374, 0x83a598, 0x3c3836, 0xb8bb26, 0xfabd2f,
                    0xfb5844, // error: #fb4934 → lighter
                ],
            ),
            "gruvbox-light" => (
                Light,
                [
                    0x3c3836, 0xfbf1c7, 0x928374, 0x076678, 0xebdbb2, 0x79740e, 0x946110,
                    0x9d0006, // warning: #b57614 → darker
                ],
            ),
            "nord" => (
                Dark,
                [
                    0xd8dee9, 0x2e3440, 0x4c566a, 0x88c0d0, 0x3b4252, 0xa3be8c, 0xebcb8b,
                    0xd08c92, // error: #bf616a → lighter
                ],
            ),
            "catppuccin-mocha" => (
                Dark,
                [
                    0xcdd6f4, 0x1e1e2e, 0x6c7086, 0x89b4fa, 0x313244, 0xa6e3a1, 0xf9e2af, 0xf38ba8,
                ],
            ),
            "catppuccin-macchiato" => (
                Dark,
                [
                    0xcad3f5, 0x24273a, 0x6e738d, 0x8aadf4, 0x363a4f, 0xa6da95, 0xeed49f, 0xed8796,
                ],
            ),
            "catppuccin-frappe" => (
                Dark,
                [
                    0xc6d0f5, 0x303446, 0x737994, 0x8caaee, 0x414559, 0xa6d189, 0xe5c890, 0xe78284,
                ],
            ),
            "catppuccin-latte" => (
                Light,
                [
                    0x4c4f69, 0xeff1f5, 0x8c8fa1, 0x1e66f5, 0xccd0da, 0x40a02b, 0x955f13,
                    0xd20f39, // warning: #df8e1d → darker
                ],
            ),
            "tokyo-night" => (
                Dark,
                [
                    0xc0caf5, 0x1a1b26, 0x565f89, 0x7aa2f7, 0x283457, 0x9ece6a, 0xe0af68, 0xf7768e,
                ],
            ),
            "tokyo-night-storm" => (
                Dark,
                [
                    0xc0caf5, 0x24283b, 0x565f89, 0x7aa2f7, 0x2e3c64, 0x9ece6a, 0xe0af68, 0xf7768e,
                ],
            ),
            "tokyo-night-day" => (
                Light,
                [
                    // Foreground/base is borderline (4.52:1); warning/error are darkened.
                    0x3760bf, 0xe1e2e7, 0x6172b0, 0x2e7de9, 0xb7c1e3, 0x587539, 0x7a5e36, 0xbd204e,
                ],
            ),
            "rose-pine" => (
                Dark,
                [
                    0xe0def4, 0x191724, 0x6e6a86, 0xc4a7e7, 0x26233a, 0x31748f, 0xf6c177, 0xeb6f92,
                ],
            ),
            "rose-pine-moon" => (
                Dark,
                [
                    0xe0def4, 0x232136, 0x6e6a86, 0xc4a7e7, 0x2a273f, 0x3e8fb0, 0xf6c177, 0xeb6f92,
                ],
            ),
            "rose-pine-dawn" => (
                Light,
                [
                    0x575279, 0xfaf4ed, 0x9893a5, 0x907aa9, 0xeadfd7, 0x286983, 0x966421,
                    0xa0586d, // warning/error darkened
                ],
            ),
            _ => return None,
        };
        let [
            foreground,
            base,
            muted,
            accent,
            selection,
            success,
            warning,
            error,
        ] = colors.map(rgb);
        // File selection uses OPI's subdued selection; folders use the accent
        // fill so their type remains distinct. Explicit ink avoids unreadable
        // foregrounds on fills, especially Solarized Light and Tokyo Night Day.
        let selected_text = if name == "solarized-dark" {
            rgb(0x93a1a1) // OPI's brighter user foreground clears 4.5:1 on selection.
        } else if contrast(foreground, selection) >= 4.5 {
            foreground
        } else {
            contrast_ink(selection)
        };
        let chip = ColorPair {
            fg: selected_text,
            bg: selection,
        };
        Some(Self {
            mode,
            foreground,
            title: warning,
            title_separator: foreground,
            accent,
            success,
            error,
            warning,
            path_error: ColorPair {
                fg: contrast_ink(error),
                bg: error,
            },
            freed_flash: ColorPair {
                fg: contrast_ink(warning),
                bg: warning,
            },
            // Only modal/tile feedback surfaces are opaque. Do not paint the terminal base.
            modal_surface: base,
            modal_text: foreground,
            tile_text: foreground,
            tile_accent: accent,
            selected_file: chip,
            selected_folder: FolderSelection {
                bg: accent,
                line1_fg: contrast_ink(accent),
                line2_fg: contrast_ink(accent),
            },
            // The small-file and empty-folder surfaces share the quieter selection fill.
            legend_chip: chip,
            empty_surface: chip,
            tile_fill: base,
            tile_border: muted,
            tile_composition: selection,
            tile_composition_track: muted,
        })
    }

    pub fn text_style(&self) -> Style {
        Self::foreground_style(self.foreground)
    }
    pub fn tile_text_style(&self) -> Style {
        Self::foreground_style(self.tile_text)
    }
    pub fn composition_styles(&self, selected: bool) -> (Style, Style) {
        if selected {
            // The track must stay readable on the selected fill: the second-line
            // ink is black in the default theme and disappears on the blue fill,
            // so use the first-line (white) ink for the unfilled track too.
            let ink = self.selected_folder.first_line_style();
            return (ink, ink);
        }
        // The reserved composition seed may be a surface tone, not readable ink.
        let ink = match (self.tile_composition, self.tile_fill) {
            (Color::Rgb(..), Color::Rgb(..))
                if contrast(self.tile_composition, self.tile_fill) < 4.5 =>
            {
                self.tile_text
            }
            _ => self.tile_composition,
        };
        (
            Self::foreground_style(ink),
            Self::foreground_style(self.tile_composition_track),
        )
    }
    pub fn legend_text_style(&self) -> Style {
        Style::default().fg(self.foreground).bg(Color::Reset)
    }
    pub fn empty_text_style(&self) -> Style {
        if self.foreground == Color::Reset {
            Style::default()
        } else {
            self.empty_surface.style()
        }
    }
    pub fn modal_style(&self, foreground: Color) -> Style {
        Style::default().fg(foreground).bg(self.modal_surface)
    }
    fn foreground_style(color: Color) -> Style {
        // Reset means inherit here, not an explicit reset patch. This preserves
        // legacy buffer layering, including black text on the empty-folder fill.
        if color == Color::Reset {
            Style::default()
        } else {
            Style::default().fg(color)
        }
    }
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

fn luminance(color: Color) -> f64 {
    let Color::Rgb(r, g, b) = color else {
        unreachable!("built-in contrast mapping only uses explicit RGB seeds")
    };
    let linear = |channel: u8| {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn contrast(a: Color, b: Color) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn contrast_ink(fill: Color) -> Color {
    // Black or white always clears 4.5:1 against an RGB fill. This is a
    // design-time RGB target, not a guarantee after terminal quantization.
    let black = rgb(0x000000);
    let white = rgb(0xffffff);
    if contrast(black, fill) >= contrast(white, fill) {
        black
    } else {
        white
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Opt;
    use clap::{CommandFactory, Parser};

    #[test]
    fn all_documented_schemes_resolve_with_explicit_modes() {
        let light = [
            "light",
            "solarized-light",
            "gruvbox-light",
            "catppuccin-latte",
            "tokyo-night-day",
            "rose-pine-dawn",
        ];
        assert_eq!(BUILTIN_SCHEMES.len(), 20); // 19 OPI palettes plus legacy default.
        for name in BUILTIN_SCHEMES {
            let theme = Theme::named(name).unwrap();
            assert_eq!(
                theme.mode,
                if light.contains(name) {
                    ThemeMode::Light
                } else {
                    ThemeMode::Dark
                },
                "{name}"
            );
            let opts = Opt::try_parse_from(["jett", "--theme", name]).unwrap();
            assert_eq!(opts.theme.as_deref(), Some(*name));
        }
        assert_eq!(Theme::named("default"), Some(Theme::default()));
        assert_eq!(Opt::try_parse_from(["jett"]).unwrap().theme, None);
    }

    #[test]
    fn unknown_names_are_rejected_without_fallback() {
        for name in ["", "unknown", "Dark", "default "] {
            assert_eq!(Theme::named(name), None);
        }
        // Custom names now resolve at the config boundary rather than inside clap.
        let opts = Opt::try_parse_from(["jett", "--theme", "unknown"]).unwrap();
        let error = crate::config::select_theme(opts.theme.as_deref(), Ok(Default::default()))
            .err()
            .unwrap();
        assert!(error.to_string().contains("unknown theme scheme 'unknown'"));
    }

    #[test]
    fn help_lists_theme_choices() {
        let help = Opt::command().render_long_help().to_string();
        assert!(help.contains("--theme <NAME>"));
        assert!(help.contains("press t to filter and preview themes"));
        assert!(help.contains("Enter saves, Esc cancels"));
        for name in BUILTIN_SCHEMES {
            assert!(help.contains(name), "missing {name} in help");
        }
    }

    #[test]
    fn helpers_preserve_styles_without_behavior_modifiers() {
        let t = Theme::named("dracula").unwrap();
        assert_eq!(t.text_style(), Style::default().fg(rgb(0xf8f8f2)));
        assert_eq!(t.tile_text_style(), t.text_style());
        assert_eq!(
            t.legend_text_style(),
            Style::default().fg(rgb(0xf8f8f2)).bg(Color::Reset)
        );
        assert_eq!(t.empty_text_style(), t.empty_surface.style());
        assert_eq!(
            t.modal_style(t.error),
            Style::default().fg(rgb(0xff5555)).bg(rgb(0x282a36))
        );
        assert_eq!(
            t.selected_file.style(),
            Style::default().fg(rgb(0xf8f8f2)).bg(rgb(0x44475a))
        );
        assert_eq!(
            t.selected_file.fill_style(),
            Style::default().fg(rgb(0x44475a)).bg(rgb(0x44475a))
        );
        assert_eq!(
            t.selected_folder.first_line_style(),
            Style::default().fg(rgb(0x000000)).bg(rgb(0xbd93f9))
        );
        assert_eq!(
            t.selected_folder.second_line_style(),
            t.selected_folder.first_line_style()
        );
        assert_eq!(
            t.selected_folder.fill_style(),
            Style::default().fg(rgb(0xbd93f9)).bg(rgb(0xbd93f9))
        );
        assert_eq!(
            t.path_error.style(),
            Style::default().fg(rgb(0x000000)).bg(rgb(0xff5555))
        );
        assert_eq!(
            t.freed_flash.style(),
            Style::default().fg(rgb(0x000000)).bg(rgb(0xf1fa8c))
        );
        assert_eq!(t.legend_chip.style(), t.empty_surface.style());
        let legacy = Theme::default();
        assert_eq!(legacy.text_style(), Style::default());
        assert_eq!(legacy.tile_text_style(), Style::default());
        assert_eq!(legacy.empty_text_style(), Style::default());
        assert_eq!(
            legacy.legend_text_style(),
            Style::default().fg(Color::Reset).bg(Color::Reset)
        );
        assert_eq!(
            (
                legacy.title,
                legacy.title_separator,
                legacy.accent,
                legacy.success,
                legacy.error,
                legacy.warning
            ),
            (
                Color::Yellow,
                Color::White,
                Color::Blue,
                Color::Green,
                Color::Red,
                Color::Yellow
            ),
        );
        assert_eq!(
            legacy.modal_style(legacy.modal_text),
            Style::default().bg(Color::Black).fg(Color::White)
        );
        assert_eq!(
            legacy.modal_style(legacy.error),
            Style::default().bg(Color::Black).fg(Color::Red)
        );
        assert_eq!(
            legacy.modal_style(legacy.warning),
            Style::default().bg(Color::Black).fg(Color::Yellow)
        );
        assert_eq!(
            legacy.legend_chip.style(),
            Style::default().bg(Color::White).fg(Color::Black)
        );
        assert_eq!(legacy.empty_surface.style(), legacy.legend_chip.style());
        assert_eq!(
            legacy.path_error.style(),
            Style::default().bg(Color::Red).fg(Color::White)
        );
        assert_eq!(
            legacy.freed_flash.style(),
            Style::default().bg(Color::Yellow).fg(Color::Black)
        );
    }

    #[test]
    fn explicit_text_on_fill_pairs_meet_design_contrast_target() {
        for name in &BUILTIN_SCHEMES[1..] {
            let t = Theme::named(name).unwrap();
            let folder = t.selected_folder;
            for (fg, bg) in [
                (t.modal_text, t.modal_surface),
                (t.error, t.modal_surface),
                (t.warning, t.modal_surface),
                (folder.line1_fg, folder.bg),
                (folder.line1_fg, folder.bg), // selected composition track ink
                (folder.line2_fg, folder.bg),
                (t.selected_file.fg, t.selected_file.bg),
                (t.path_error.fg, t.path_error.bg),
                (t.freed_flash.fg, t.freed_flash.bg),
                (t.legend_chip.fg, t.legend_chip.bg),
                (t.empty_surface.fg, t.empty_surface.bg),
            ] {
                assert!(contrast(fg, bg) >= 4.5, "{name}: {fg:?} on {bg:?}");
            }
        }
    }
}
