//! The Last Light look: warm charcoal surfaces lit by one amber accent, like
//! the last lamp still burning. Every screen draws from these colors and
//! helpers, so restyling happens here rather than in each mini app.

use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Theme,
};

/// Behind everything: the central area and text fields.
pub const BG: Color32 = Color32::from_rgb(0x14, 0x12, 0x10);
/// Header and side panels.
pub const PANEL: Color32 = Color32::from_rgb(0x1b, 0x18, 0x15);
/// Cards, buttons and other raised surfaces.
pub const RAISED: Color32 = Color32::from_rgb(0x25, 0x21, 0x1c);
pub const HOVER: Color32 = Color32::from_rgb(0x2f, 0x2a, 0x24);
pub const BORDER: Color32 = Color32::from_rgb(0x33, 0x2d, 0x26);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x4a, 0x41, 0x37);

pub const TEXT: Color32 = Color32::from_rgb(0xec, 0xe4, 0xd8);
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xfb, 0xf6, 0xee);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xa8, 0x9d, 0x8e);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x72, 0x69, 0x5d);

pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xa9, 0x3b);
/// The accent at low strength, pre-blended over `PANEL`: selected rows, badges,
/// the user's chat bubbles.
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x3b, 0x2e, 0x1b);
/// Dark text for use on top of a solid `ACCENT` fill.
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x1f, 0x16, 0x08);

pub const WARN: Color32 = Color32::from_rgb(0xe8, 0xb8, 0x5c);
pub const ERROR: Color32 = Color32::from_rgb(0xe8, 0x6a, 0x52);
/// Fill for destructive buttons such as Delete.
pub const DANGER: Color32 = Color32::from_rgb(0x9a, 0x37, 0x26);

const RADIUS: u8 = 6;

/// Installs the theme. The app is always dark: a bright screen is a beacon
/// and a battery drain, both bad when the lights are out.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(20.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(11.5, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.5, FontFamily::Monospace),
            ),
        ]
        .into();

        let spacing = &mut style.spacing;
        spacing.item_spacing = egui::vec2(8.0, 6.0);
        spacing.button_padding = egui::vec2(10.0, 5.0);
        spacing.interact_size.y = 24.0;
        spacing.menu_margin = Margin::same(6);
        spacing.window_margin = Margin::same(16);

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = PANEL;
        v.window_fill = PANEL;
        v.extreme_bg_color = BG;
        v.text_edit_bg_color = Some(BG);
        v.faint_bg_color = RAISED;
        v.code_bg_color = RAISED;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARN;
        v.error_fg_color = ERROR;
        v.selection.bg_fill = ACCENT_SOFT;
        v.selection.stroke = Stroke::new(1.0, ACCENT);
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(8);
        let shadow = Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(140),
        };
        v.window_shadow = shadow;
        v.popup_shadow = shadow;

        let w = &mut v.widgets;
        // Labels and separators.
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.weak_bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        // Buttons at rest, hovered, pressed, and with an open menu.
        for (state, fill, stroke, text) in [
            (&mut w.inactive, RAISED, Stroke::NONE, TEXT),
            (
                &mut w.hovered,
                HOVER,
                Stroke::new(1.0, BORDER_STRONG),
                TEXT_STRONG,
            ),
            (
                &mut w.active,
                BORDER_STRONG,
                Stroke::new(1.0, ACCENT),
                TEXT_STRONG,
            ),
            (
                &mut w.open,
                HOVER,
                Stroke::new(1.0, BORDER_STRONG),
                TEXT_STRONG,
            ),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = stroke;
            state.fg_stroke = Stroke::new(1.0, text);
            state.expansion = 0.0;
        }
        for state in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            state.corner_radius = CornerRadius::same(RADIUS);
        }
    });
}

/// A small amber light with a soft glow: the app's mark.
pub fn lamp(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    for (radius, color, alpha) in LAMP_LAYERS {
        let [r, g, b, _] = color.to_array();
        let fill = Color32::from_rgba_unmultiplied(r, g, b, alpha);
        ui.painter()
            .circle_filled(rect.center(), size * radius, fill);
    }
}

/// The lamp's circles, painted in order from the outer glow to the core:
/// radius as a fraction of the lamp's size, color, and opacity (0-255). The
/// opacity is kept separate from the color so the logo files get the exact
/// colors. The logo files in `branding/` are generated from this table (see
/// `render_logo`), so change it here and regenerate them.
pub const LAMP_LAYERS: [(f32, Color32, u8); 4] = [
    (0.5, ACCENT, 30),
    (0.38, ACCENT, 60),
    (0.28, ACCENT, 110),
    (0.18, Color32::from_rgb(0xff, 0xdc, 0x9a), 255),
];

/// A small, muted, upper-case heading for a sidebar section.
pub fn section_label(text: &str) -> egui::RichText {
    egui::RichText::new(text.to_uppercase())
        .small()
        .strong()
        .color(TEXT_MUTED)
}

/// A frameless icon-style button, e.g. "+" next to a section heading.
pub fn quiet_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(TEXT_MUTED))
        .frame_when_inactive(false)
}

/// A solid accent button for the main action on a screen.
pub fn primary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text.to_owned())
            .strong()
            .color(ON_ACCENT),
    )
    .fill(ACCENT)
}

/// A row of mutually exclusive options drawn as one pill, like Split/Edit/View.
/// Returns the option the user clicked, if any.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    enabled: bool,
    current: T,
    options: &[(T, &str)],
) -> Option<T> {
    let mut picked = None;
    egui::Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(RADIUS + 2))
        .inner_margin(Margin::same(2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut add = |ui: &mut egui::Ui, &(value, label): &(T, &str)| {
                let selected = value == current;
                let color = if selected { ACCENT } else { TEXT_MUTED };
                let button =
                    egui::Button::selectable(selected, egui::RichText::new(label).color(color));
                if ui.add_enabled(enabled, button).clicked() {
                    picked = Some(value);
                }
            };
            // Inside a right-to-left row, add the options in reverse so they
            // still read left to right. (A nested left-to-right layout would
            // stretch the pill across the whole row.)
            if ui.layout().prefer_right_to_left() {
                options.iter().rev().for_each(|option| add(ui, option));
            } else {
                ui.horizontal(|ui| options.iter().for_each(|option| add(ui, option)));
            }
        });
    picked
}

/// The lamp as straight-alpha RGBA pixels on a transparent background, `size`
/// by `size`: the window icon and the PNG logo files. Composites the circles
/// with "over" blending in premultiplied sRGB, as egui and SVG renderers do,
/// supersampling each pixel for smooth edges.
pub fn lamp_rgba(size: u32) -> Vec<u8> {
    const SUPERSAMPLING: u32 = 8;
    let center = size as f32 / 2.0;
    let samples = (SUPERSAMPLING * SUPERSAMPLING) as f32;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut out = [0.0f32; 4]; // premultiplied, 0..=1
            for (radius, color, alpha) in LAMP_LAYERS {
                let radius = radius * size as f32;
                let mut inside = 0;
                for sy in 0..SUPERSAMPLING {
                    for sx in 0..SUPERSAMPLING {
                        let px = x as f32 + (sx as f32 + 0.5) / SUPERSAMPLING as f32 - center;
                        let py = y as f32 + (sy as f32 + 0.5) / SUPERSAMPLING as f32 - center;
                        inside += u32::from(px * px + py * py <= radius * radius);
                    }
                }
                let alpha = f32::from(alpha) / 255.0 * (inside as f32 / samples);
                let [r, g, b, _] = color.to_array().map(|c| f32::from(c) / 255.0);
                let src = [r * alpha, g * alpha, b * alpha, alpha];
                for i in 0..4 {
                    out[i] = src[i] + out[i] * (1.0 - src[3]);
                }
            }
            let alpha = out[3];
            let channel = |c: f32| {
                let straight = if alpha > 0.0 { c / alpha } else { 0.0 };
                (straight * 255.0).round().clamp(0.0, 255.0) as u8
            };
            pixels.extend([
                channel(out[0]),
                channel(out[1]),
                channel(out[2]),
                (alpha * 255.0).round() as u8,
            ]);
        }
    }
    pixels
}

#[cfg(test)]
mod logo_files {
    use super::*;

    const PNG_SIZES: [u32; 8] = [16, 32, 48, 64, 128, 256, 512, 1024];

    /// Writes the lamp logo to `branding/` as an SVG and as PNGs at common
    /// icon sizes, on a transparent background. Both are drawn from
    /// `LAMP_LAYERS`, the same table the app paints with.
    ///
    /// `cargo test render_logo -- --ignored`
    #[test]
    #[ignore = "writes files into branding/; run by hand after changing the lamp"]
    fn render_logo() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("branding");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("logo.svg"), svg()).unwrap();
        for size in PNG_SIZES {
            image::RgbaImage::from_raw(size, size, lamp_rgba(size))
                .unwrap()
                .save(dir.join(format!("logo-{size}.png")))
                .unwrap();
        }
    }

    fn svg() -> String {
        let circles: String = LAMP_LAYERS
            .iter()
            .map(|&(radius, color, alpha)| {
                let [r, g, b, _] = color.to_array();
                format!(
                    "  <circle cx=\"50\" cy=\"50\" r=\"{}\" fill=\"#{r:02x}{g:02x}{b:02x}\" fill-opacity=\"{:.3}\"/>\n",
                    radius * 100.0,
                    f32::from(alpha) / 255.0
                )
            })
            .collect();
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\" width=\"512\" height=\"512\">\n\
             <title>Last Light</title>\n{circles}</svg>\n"
        )
    }
}
