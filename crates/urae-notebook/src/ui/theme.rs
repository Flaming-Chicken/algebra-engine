//! # Modernized Theming Subsystem for URAE Notebook
//!
//! Provides customizable, high-contrast, accessible themes with tailored math syntax highlighting,
//! refined widget geometry, and custom `egui::Visuals`.

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Vec2, Visuals};
use serde::{Deserialize, Serialize};

/// Supported theme presets in URAE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeKind {
    #[default]
    Dark,
    Light,
    CatppuccinMocha,
    Nord,
    Dracula,
    SolarizedDark,
    SolarizedLight,
    Cyberpunk,
    HighContrastDark,
    HighContrastLight,
}

impl ThemeKind {
    /// List all available theme kinds.
    pub fn all() -> &'static [ThemeKind] {
        &[
            ThemeKind::Dark,
            ThemeKind::Light,
            ThemeKind::CatppuccinMocha,
            ThemeKind::Nord,
            ThemeKind::Dracula,
            ThemeKind::SolarizedDark,
            ThemeKind::SolarizedLight,
            ThemeKind::Cyberpunk,
            ThemeKind::HighContrastDark,
            ThemeKind::HighContrastLight,
        ]
    }

    /// User-facing display name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Dark => "🌙 Charcoal Dark",
            Self::Light => "☀️ Slate Paper Light",
            Self::CatppuccinMocha => "☕ Catppuccin Mocha",
            Self::Nord => "❄️ Nord Arctic",
            Self::Dracula => "🧛 Dracula Neon",
            Self::SolarizedDark => "🌌 Solarized Dark",
            Self::SolarizedLight => "📜 Solarized Light",
            Self::Cyberpunk => "⚡ Cyberpunk Matrix",
            Self::HighContrastDark => "👁️ High Contrast Dark (WCAG AAA)",
            Self::HighContrastLight => "👓 High Contrast Light (WCAG AAA)",
        }
    }

    /// Check if the theme is dark.
    pub fn is_dark(&self) -> bool {
        !matches!(
            self,
            Self::Light | Self::SolarizedLight | Self::HighContrastLight
        )
    }
}

/// Semantic color palette for mathematical notation, graphs, and UI accents.
#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_card: Color32,
    pub bg_input: Color32,
    pub border: Color32,
    pub accent_primary: Color32,
    pub accent_secondary: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub text_success: Color32,
    pub text_warning: Color32,
    pub text_error: Color32,

    // Mathematical syntax colors
    pub math_var: Color32,
    pub math_number: Color32,
    pub math_operator: Color32,
    pub math_keyword: Color32,
    pub math_result: Color32,
    pub math_comment: Color32,

    // Plot line colors
    pub plot_palette: [Color32; 6],
}

impl ThemeKind {
    /// Get the rich color palette for this theme.
    pub fn palette(&self) -> ThemePalette {
        match self {
            Self::Dark => ThemePalette {
                bg_app: Color32::from_rgb(18, 20, 26),
                bg_panel: Color32::from_rgb(24, 28, 36),
                bg_card: Color32::from_rgb(32, 36, 48),
                bg_input: Color32::from_rgb(14, 16, 22),
                border: Color32::from_rgb(45, 52, 68),
                accent_primary: Color32::from_rgb(86, 156, 255),
                accent_secondary: Color32::from_rgb(175, 110, 255),
                text_primary: Color32::from_rgb(240, 244, 250),
                text_muted: Color32::from_rgb(140, 150, 170),
                text_success: Color32::from_rgb(80, 220, 140),
                text_warning: Color32::from_rgb(255, 190, 70),
                text_error: Color32::from_rgb(255, 90, 95),
                math_var: Color32::from_rgb(130, 210, 255),
                math_number: Color32::from_rgb(255, 170, 90),
                math_operator: Color32::from_rgb(255, 130, 180),
                math_keyword: Color32::from_rgb(190, 140, 255),
                math_result: Color32::from_rgb(100, 230, 160),
                math_comment: Color32::from_rgb(100, 115, 135),
                plot_palette: [
                    Color32::from_rgb(86, 156, 255),
                    Color32::from_rgb(255, 110, 150),
                    Color32::from_rgb(80, 220, 140),
                    Color32::from_rgb(255, 190, 70),
                    Color32::from_rgb(175, 110, 255),
                    Color32::from_rgb(80, 230, 230),
                ],
            },
            Self::Light => ThemePalette {
                bg_app: Color32::from_rgb(248, 250, 252),
                bg_panel: Color32::from_rgb(241, 245, 249),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_input: Color32::from_rgb(255, 255, 255),
                border: Color32::from_rgb(203, 213, 225),
                accent_primary: Color32::from_rgb(37, 99, 235),
                accent_secondary: Color32::from_rgb(147, 51, 234),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_muted: Color32::from_rgb(100, 116, 139),
                text_success: Color32::from_rgb(22, 163, 74),
                text_warning: Color32::from_rgb(217, 119, 6),
                text_error: Color32::from_rgb(220, 38, 38),
                math_var: Color32::from_rgb(2, 132, 199),
                math_number: Color32::from_rgb(217, 119, 6),
                math_operator: Color32::from_rgb(219, 39, 119),
                math_keyword: Color32::from_rgb(124, 58, 237),
                math_result: Color32::from_rgb(13, 148, 136),
                math_comment: Color32::from_rgb(148, 163, 184),
                plot_palette: [
                    Color32::from_rgb(37, 99, 235),
                    Color32::from_rgb(219, 39, 119),
                    Color32::from_rgb(13, 148, 136),
                    Color32::from_rgb(217, 119, 6),
                    Color32::from_rgb(124, 58, 237),
                    Color32::from_rgb(2, 132, 199),
                ],
            },
            Self::CatppuccinMocha => ThemePalette {
                bg_app: Color32::from_rgb(30, 30, 46),
                bg_panel: Color32::from_rgb(24, 24, 37),
                bg_card: Color32::from_rgb(49, 50, 68),
                bg_input: Color32::from_rgb(17, 17, 27),
                border: Color32::from_rgb(69, 71, 90),
                accent_primary: Color32::from_rgb(137, 180, 250),
                accent_secondary: Color32::from_rgb(203, 166, 247),
                text_primary: Color32::from_rgb(205, 214, 244),
                text_muted: Color32::from_rgb(147, 153, 178),
                text_success: Color32::from_rgb(166, 227, 161),
                text_warning: Color32::from_rgb(249, 226, 175),
                text_error: Color32::from_rgb(243, 139, 168),
                math_var: Color32::from_rgb(137, 220, 235),
                math_number: Color32::from_rgb(250, 179, 135),
                math_operator: Color32::from_rgb(245, 194, 231),
                math_keyword: Color32::from_rgb(203, 166, 247),
                math_result: Color32::from_rgb(148, 226, 213),
                math_comment: Color32::from_rgb(108, 112, 134),
                plot_palette: [
                    Color32::from_rgb(137, 180, 250),
                    Color32::from_rgb(243, 139, 168),
                    Color32::from_rgb(166, 227, 161),
                    Color32::from_rgb(249, 226, 175),
                    Color32::from_rgb(203, 166, 247),
                    Color32::from_rgb(148, 226, 213),
                ],
            },
            Self::Nord => ThemePalette {
                bg_app: Color32::from_rgb(46, 52, 64),
                bg_panel: Color32::from_rgb(36, 41, 51),
                bg_card: Color32::from_rgb(59, 66, 82),
                bg_input: Color32::from_rgb(46, 52, 64),
                border: Color32::from_rgb(76, 86, 106),
                accent_primary: Color32::from_rgb(136, 192, 208),
                accent_secondary: Color32::from_rgb(129, 161, 193),
                text_primary: Color32::from_rgb(236, 239, 244),
                text_muted: Color32::from_rgb(148, 163, 184),
                text_success: Color32::from_rgb(163, 190, 140),
                text_warning: Color32::from_rgb(235, 203, 139),
                text_error: Color32::from_rgb(191, 97, 106),
                math_var: Color32::from_rgb(143, 188, 187),
                math_number: Color32::from_rgb(208, 135, 112),
                math_operator: Color32::from_rgb(180, 142, 173),
                math_keyword: Color32::from_rgb(129, 161, 193),
                math_result: Color32::from_rgb(163, 190, 140),
                math_comment: Color32::from_rgb(94, 129, 172),
                plot_palette: [
                    Color32::from_rgb(136, 192, 208),
                    Color32::from_rgb(191, 97, 106),
                    Color32::from_rgb(163, 190, 140),
                    Color32::from_rgb(235, 203, 139),
                    Color32::from_rgb(180, 142, 173),
                    Color32::from_rgb(143, 188, 187),
                ],
            },
            Self::Dracula => ThemePalette {
                bg_app: Color32::from_rgb(40, 42, 54),
                bg_panel: Color32::from_rgb(33, 34, 44),
                bg_card: Color32::from_rgb(68, 71, 90),
                bg_input: Color32::from_rgb(25, 26, 33),
                border: Color32::from_rgb(98, 114, 164),
                accent_primary: Color32::from_rgb(189, 147, 249),
                accent_secondary: Color32::from_rgb(255, 121, 198),
                text_primary: Color32::from_rgb(248, 248, 242),
                text_muted: Color32::from_rgb(98, 114, 164),
                text_success: Color32::from_rgb(80, 250, 123),
                text_warning: Color32::from_rgb(241, 250, 140),
                text_error: Color32::from_rgb(255, 85, 85),
                math_var: Color32::from_rgb(139, 233, 253),
                math_number: Color32::from_rgb(255, 184, 108),
                math_operator: Color32::from_rgb(255, 121, 198),
                math_keyword: Color32::from_rgb(189, 147, 249),
                math_result: Color32::from_rgb(80, 250, 123),
                math_comment: Color32::from_rgb(98, 114, 164),
                plot_palette: [
                    Color32::from_rgb(189, 147, 249),
                    Color32::from_rgb(255, 121, 198),
                    Color32::from_rgb(80, 250, 123),
                    Color32::from_rgb(255, 184, 108),
                    Color32::from_rgb(139, 233, 253),
                    Color32::from_rgb(241, 250, 140),
                ],
            },
            Self::SolarizedDark => ThemePalette {
                bg_app: Color32::from_rgb(0, 43, 54),
                bg_panel: Color32::from_rgb(7, 54, 66),
                bg_card: Color32::from_rgb(10, 68, 82),
                bg_input: Color32::from_rgb(0, 30, 38),
                border: Color32::from_rgb(88, 110, 117),
                accent_primary: Color32::from_rgb(38, 139, 210),
                accent_secondary: Color32::from_rgb(108, 113, 196),
                text_primary: Color32::from_rgb(147, 161, 161),
                text_muted: Color32::from_rgb(101, 123, 131),
                text_success: Color32::from_rgb(133, 153, 0),
                text_warning: Color32::from_rgb(181, 137, 0),
                text_error: Color32::from_rgb(220, 50, 47),
                math_var: Color32::from_rgb(42, 161, 152),
                math_number: Color32::from_rgb(203, 75, 22),
                math_operator: Color32::from_rgb(211, 54, 130),
                math_keyword: Color32::from_rgb(108, 113, 196),
                math_result: Color32::from_rgb(133, 153, 0),
                math_comment: Color32::from_rgb(88, 110, 117),
                plot_palette: [
                    Color32::from_rgb(38, 139, 210),
                    Color32::from_rgb(211, 54, 130),
                    Color32::from_rgb(133, 153, 0),
                    Color32::from_rgb(181, 137, 0),
                    Color32::from_rgb(108, 113, 196),
                    Color32::from_rgb(42, 161, 152),
                ],
            },
            Self::SolarizedLight => ThemePalette {
                bg_app: Color32::from_rgb(253, 246, 227),
                bg_panel: Color32::from_rgb(238, 232, 213),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_input: Color32::from_rgb(255, 255, 255),
                border: Color32::from_rgb(200, 195, 178),
                accent_primary: Color32::from_rgb(38, 139, 210),
                accent_secondary: Color32::from_rgb(108, 113, 196),
                text_primary: Color32::from_rgb(101, 123, 131),
                text_muted: Color32::from_rgb(147, 161, 161),
                text_success: Color32::from_rgb(133, 153, 0),
                text_warning: Color32::from_rgb(181, 137, 0),
                text_error: Color32::from_rgb(220, 50, 47),
                math_var: Color32::from_rgb(42, 161, 152),
                math_number: Color32::from_rgb(203, 75, 22),
                math_operator: Color32::from_rgb(211, 54, 130),
                math_keyword: Color32::from_rgb(108, 113, 196),
                math_result: Color32::from_rgb(133, 153, 0),
                math_comment: Color32::from_rgb(147, 161, 161),
                plot_palette: [
                    Color32::from_rgb(38, 139, 210),
                    Color32::from_rgb(211, 54, 130),
                    Color32::from_rgb(133, 153, 0),
                    Color32::from_rgb(181, 137, 0),
                    Color32::from_rgb(108, 113, 196),
                    Color32::from_rgb(42, 161, 152),
                ],
            },
            Self::Cyberpunk => ThemePalette {
                bg_app: Color32::from_rgb(13, 17, 23),
                bg_panel: Color32::from_rgb(18, 24, 32),
                bg_card: Color32::from_rgb(26, 35, 48),
                bg_input: Color32::from_rgb(8, 12, 16),
                border: Color32::from_rgb(0, 255, 170),
                accent_primary: Color32::from_rgb(0, 255, 170),
                accent_secondary: Color32::from_rgb(255, 0, 128),
                text_primary: Color32::from_rgb(220, 255, 240),
                text_muted: Color32::from_rgb(0, 180, 120),
                text_success: Color32::from_rgb(0, 255, 170),
                text_warning: Color32::from_rgb(255, 230, 0),
                text_error: Color32::from_rgb(255, 0, 100),
                math_var: Color32::from_rgb(0, 230, 255),
                math_number: Color32::from_rgb(255, 230, 0),
                math_operator: Color32::from_rgb(255, 0, 128),
                math_keyword: Color32::from_rgb(180, 0, 255),
                math_result: Color32::from_rgb(0, 255, 170),
                math_comment: Color32::from_rgb(0, 100, 70),
                plot_palette: [
                    Color32::from_rgb(0, 255, 170),
                    Color32::from_rgb(255, 0, 128),
                    Color32::from_rgb(0, 230, 255),
                    Color32::from_rgb(255, 230, 0),
                    Color32::from_rgb(180, 0, 255),
                    Color32::from_rgb(255, 80, 0),
                ],
            },
            Self::HighContrastDark => ThemePalette {
                bg_app: Color32::BLACK,
                bg_panel: Color32::from_rgb(12, 12, 12),
                bg_card: Color32::from_rgb(20, 20, 20),
                bg_input: Color32::BLACK,
                border: Color32::WHITE,
                accent_primary: Color32::from_rgb(0, 255, 255),
                accent_secondary: Color32::from_rgb(255, 255, 0),
                text_primary: Color32::WHITE,
                text_muted: Color32::from_rgb(200, 200, 200),
                text_success: Color32::from_rgb(0, 255, 0),
                text_warning: Color32::from_rgb(255, 255, 0),
                text_error: Color32::from_rgb(255, 50, 50),
                math_var: Color32::from_rgb(0, 255, 255),
                math_number: Color32::from_rgb(255, 255, 0),
                math_operator: Color32::WHITE,
                math_keyword: Color32::from_rgb(255, 100, 255),
                math_result: Color32::from_rgb(0, 255, 0),
                math_comment: Color32::from_rgb(160, 160, 160),
                plot_palette: [
                    Color32::from_rgb(0, 255, 255),
                    Color32::from_rgb(255, 255, 0),
                    Color32::from_rgb(0, 255, 0),
                    Color32::from_rgb(255, 100, 255),
                    Color32::WHITE,
                    Color32::from_rgb(255, 128, 0),
                ],
            },
            Self::HighContrastLight => ThemePalette {
                bg_app: Color32::WHITE,
                bg_panel: Color32::from_rgb(245, 245, 245),
                bg_card: Color32::WHITE,
                bg_input: Color32::WHITE,
                border: Color32::BLACK,
                accent_primary: Color32::from_rgb(0, 0, 200),
                accent_secondary: Color32::from_rgb(160, 0, 160),
                text_primary: Color32::BLACK,
                text_muted: Color32::from_rgb(60, 60, 60),
                text_success: Color32::from_rgb(0, 140, 0),
                text_warning: Color32::from_rgb(180, 100, 0),
                text_error: Color32::from_rgb(200, 0, 0),
                math_var: Color32::from_rgb(0, 0, 200),
                math_number: Color32::from_rgb(160, 60, 0),
                math_operator: Color32::BLACK,
                math_keyword: Color32::from_rgb(140, 0, 140),
                math_result: Color32::from_rgb(0, 130, 0),
                math_comment: Color32::from_rgb(90, 90, 90),
                plot_palette: [
                    Color32::from_rgb(0, 0, 200),
                    Color32::from_rgb(200, 0, 0),
                    Color32::from_rgb(0, 140, 0),
                    Color32::from_rgb(160, 0, 160),
                    Color32::BLACK,
                    Color32::from_rgb(200, 100, 0),
                ],
            },
        }
    }

    /// Construct modern `egui::Visuals` for this theme.
    pub fn to_visuals(&self) -> Visuals {
        let p = self.palette();
        let is_dark = self.is_dark();

        let mut visuals = if is_dark {
            Visuals::dark()
        } else {
            Visuals::light()
        };

        // Window & Panel Backgrounds
        visuals.window_fill = p.bg_card;
        visuals.panel_fill = p.bg_panel;
        visuals.faint_bg_color = p.bg_app;
        visuals.extreme_bg_color = p.bg_input;
        visuals.code_bg_color = p.bg_input;

        // Modern Rounded Corners (6–8px)
        visuals.window_corner_radius = CornerRadius::same(8);
        visuals.menu_corner_radius = CornerRadius::same(6);

        // Window & Dialog Strokes
        visuals.window_stroke = Stroke::new(1.0_f32, p.border);

        // Widget Normal State
        visuals.widgets.noninteractive.bg_fill = p.bg_panel;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        visuals.widgets.inactive.bg_fill = p.bg_card;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

        // Widget Hover State
        visuals.widgets.hovered.bg_fill = p.accent_primary.linear_multiply(0.25);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, p.accent_primary);
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, p.text_primary);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        visuals.widgets.hovered.expansion = 1.0_f32;

        // Widget Active / Clicked State
        visuals.widgets.active.bg_fill = p.accent_primary.linear_multiply(0.4);
        visuals.widgets.active.bg_stroke = Stroke::new(2.0_f32, p.accent_primary);
        visuals.widgets.active.fg_stroke = Stroke::new(2.0_f32, Color32::WHITE);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);
        visuals.widgets.active.expansion = 0.5_f32;

        // Selection Highlighting
        visuals.selection.bg_fill = p.accent_primary.linear_multiply(0.35);
        visuals.selection.stroke = Stroke::new(1.5_f32, p.accent_primary);

        visuals
    }

    /// Construct ergonomic spacing and layout `egui::Style`.
    pub fn to_style(&self) -> Style {
        let mut style = Style {
            visuals: self.to_visuals(),
            ..Default::default()
        };

        // Modern spacing & padding metrics (WCAG 2.5.5 / 2.5.8 compliant touch targets)
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.interact_size = if matches!(
            self,
            ThemeKind::HighContrastDark | ThemeKind::HighContrastLight
        ) {
            Vec2::new(44.0, 44.0)
        } else {
            Vec2::new(36.0, 32.0)
        };
        style.spacing.button_padding = Vec2::new(12.0, 8.0);
        style.spacing.window_margin = Margin::same(12);
        style.spacing.menu_margin = Margin::same(8);
        style.spacing.indent = 16.0;
        style.spacing.scroll.bar_width = 8.0;
        style.spacing.scroll.bar_inner_margin = 2.0;

        style
    }
}

impl ThemePalette {
    /// Tokenize and layout continuous math editor text with rich semantic syntax highlighting.
    pub fn math_syntax_layouter(
        &self,
        ui: &egui::Ui,
        text: &str,
        wrap_width: f32,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;

        for (line_idx, line) in text.split('\n').enumerate() {
            if line_idx > 0 {
                job.append(
                    "\n",
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color: self.text_primary,
                        ..Default::default()
                    },
                );
            }

            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                job.append(
                    line,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(14.0),
                        color: self.accent_primary,
                        ..Default::default()
                    },
                );
                continue;
            }

            if trimmed.starts_with("//") {
                job.append(
                    line,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color: self.math_comment,
                        ..Default::default()
                    },
                );
                continue;
            }

            // Tokenize line characters with zero heap allocations (slicing &str directly)
            let mut i = 0;
            while i < line.len() {
                let remaining = &line[i..];
                let Some(c) = remaining.chars().next() else {
                    break;
                };

                if c.is_whitespace() {
                    let start = i;
                    while i < line.len() {
                        let Some(ch) = line[i..].chars().next() else {
                            break;
                        };
                        if ch.is_whitespace() {
                            i += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    job.append(
                        &line[start..i],
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(13.0),
                            color: self.text_primary,
                            ..Default::default()
                        },
                    );
                } else if c.is_ascii_digit()
                    || (c == '.'
                        && line[i + 1..]
                            .chars()
                            .next()
                            .is_some_and(|next_c| next_c.is_ascii_digit()))
                {
                    let start = i;
                    while i < line.len() {
                        let Some(ch) = line[i..].chars().next() else {
                            break;
                        };
                        if ch.is_ascii_digit() || ch == '.' {
                            i += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    job.append(
                        &line[start..i],
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(13.0),
                            color: self.math_number,
                            ..Default::default()
                        },
                    );
                } else if c.is_alphabetic() || c == '_' {
                    let start = i;
                    while i < line.len() {
                        let Some(ch) = line[i..].chars().next() else {
                            break;
                        };
                        if ch.is_alphanumeric() || ch == '_' {
                            i += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    let ident = &line[start..i];
                    let color = match ident {
                        "solve" | "diff" | "integrate" | "series" | "solve_ode" | "matrix"
                        | "in" | "Reals" | "Integers" | "Positive" | "NonNegative" | "Complex"
                        | "Parameter" | "Variable" => self.math_keyword,
                        "sin" | "cos" | "tan" | "exp" | "ln" | "log" | "sqrt" | "abs" | "det"
                        | "inv" | "trace" | "eigenvalues" | "cross" | "dot" => {
                            self.accent_secondary
                        }
                        "ans" => self.accent_primary,
                        _ => self.math_var,
                    };
                    job.append(
                        ident,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(13.0),
                            color,
                            ..Default::default()
                        },
                    );
                } else if c == '$'
                    && line[i + 1..]
                        .chars()
                        .next()
                        .is_some_and(|next_c| next_c.is_ascii_digit())
                {
                    let start = i;
                    i += 1;
                    while i < line.len() {
                        let Some(ch) = line[i..].chars().next() else {
                            break;
                        };
                        if ch.is_ascii_digit() {
                            i += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    job.append(
                        &line[start..i],
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(13.0),
                            color: self.accent_primary,
                            ..Default::default()
                        },
                    );
                } else {
                    let start = i;
                    i += c.len_utf8();
                    let color = match c {
                        '+' | '-' | '*' | '/' | '^' | '=' | '<' | '>' => self.math_operator,
                        '(' | ')' | '[' | ']' | '{' | '}' | '|' | ',' | ';' => self.text_muted,
                        _ => self.text_primary,
                    };
                    job.append(
                        &line[start..i],
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(13.0),
                            color,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        ui.fonts_mut(|f| f.layout_job(job))
    }

    /// Layout plain text in monospace font without semantic syntax highlighting.
    pub fn plain_monospace_layouter(
        &self,
        ui: &egui::Ui,
        text: &str,
        wrap_width: f32,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(13.0),
                color: self.text_primary,
                ..Default::default()
            },
        );
        ui.fonts_mut(|f| f.layout_job(job))
    }
}
