mod apps;
mod storage;
mod theme;

use apps::MiniApp;
use eframe::egui;

const CARD_MIN_WIDTH: f32 = 220.0;
const CARD_HEIGHT: f32 = 136.0;
const CARD_GAP: f32 = 16.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Last Light")
            .with_inner_size([1024.0, 720.0])
            .with_min_inner_size([640.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Last Light",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(LastLightApp::new()))
        }),
    )
}

struct LastLightApp {
    apps: Vec<Box<dyn MiniApp>>,
    /// Index into `apps` of the open app; `None` shows the card grid.
    open: Option<usize>,
}

impl LastLightApp {
    fn new() -> Self {
        Self {
            apps: apps::all(),
            open: None,
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(i) = self.open {
                let back =
                    egui::Button::new(egui::RichText::new("< Back").color(theme::TEXT_MUTED))
                        .frame_when_inactive(false);
                if ui.add(back).clicked() {
                    self.apps[i].on_close();
                    self.open = None;
                }
                ui.separator();
            }
            theme::lamp(ui, 22.0);
            let title = |text: &str| egui::RichText::new(text).size(17.0).strong();
            match self.open {
                Some(i) => {
                    ui.label(title(self.apps[i].name()).color(theme::TEXT_STRONG));
                    ui.label(title("| Last Light").color(theme::TEXT_FAINT));
                }
                None => {
                    ui.label(title("Last Light").color(theme::TEXT_STRONG));
                }
            }
        });
    }

    fn card_grid(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Your toolkit")
                        .size(24.0)
                        .strong()
                        .color(theme::TEXT_STRONG),
                );
                ui.label(
                    egui::RichText::new("Everything here runs on this laptop. No internet needed.")
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(20.0);
                // Stretch the cards so each row fills the width instead of
                // leaving a gap on the right.
                let width = ui.available_width();
                let columns = ((width + CARD_GAP) / (CARD_MIN_WIDTH + CARD_GAP))
                    .floor()
                    .max(1.0);
                let card_width = ((width - CARD_GAP * (columns - 1.0)) / columns).floor();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(CARD_GAP, CARD_GAP);
                    for (i, app) in self.apps.iter().enumerate() {
                        if card(ui, app.as_ref(), card_width).clicked() {
                            self.open = Some(i);
                        }
                    }
                });
                ui.add_space(16.0);
            });
    }
}

/// A clickable card for one mini app: a letter badge, its name and a short
/// description. Apps that aren't built yet are dimmed and marked "Soon".
fn card(ui: &mut egui::Ui, app: &dyn MiniApp, width: f32) -> egui::Response {
    let size = egui::vec2(width, CARD_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let ready = app.is_ready();
    let hovered = response.hovered();
    let painter = ui.painter();

    let (fill, border) = match (hovered, ready) {
        (true, true) => (theme::HOVER, theme::ACCENT),
        (true, false) => (theme::HOVER, theme::BORDER_STRONG),
        (false, _) => (theme::RAISED, theme::BORDER),
    };
    painter.rect(
        rect,
        10.0,
        fill,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    // Text is painted rather than added as widgets, so it can't steal the card's clicks.
    let inner = rect.shrink(18.0);
    let badge = egui::Rect::from_min_size(inner.left_top(), egui::vec2(34.0, 34.0));
    let (badge_fill, badge_text) = if ready {
        (theme::ACCENT_SOFT, theme::ACCENT)
    } else {
        (theme::PANEL, theme::TEXT_FAINT)
    };
    painter.rect_filled(badge, 8.0, badge_fill);
    let initial = app.name().chars().next().unwrap_or('?').to_string();
    painter.text(
        badge.center(),
        egui::Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(17.0),
        badge_text,
    );

    if !ready {
        let soon = painter.layout_no_wrap(
            "Soon".to_owned(),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
        let pill = egui::Rect::from_min_size(
            egui::pos2(inner.right() - soon.size().x - 16.0, inner.top() + 6.0),
            soon.size() + egui::vec2(16.0, 6.0),
        );
        painter.rect(
            pill,
            10.0,
            theme::PANEL,
            egui::Stroke::new(1.0, theme::BORDER),
            egui::StrokeKind::Inside,
        );
        painter.galley(pill.min + egui::vec2(8.0, 3.0), soon, theme::TEXT_MUTED);
    }

    let title_color = if ready {
        theme::TEXT_STRONG
    } else {
        theme::TEXT_MUTED
    };
    let title = painter.layout(
        app.name().to_owned(),
        egui::FontId::proportional(17.0),
        title_color,
        inner.width(),
    );
    let description = painter.layout(
        app.description().to_owned(),
        egui::FontId::proportional(13.5),
        if ready {
            theme::TEXT_MUTED
        } else {
            theme::TEXT_FAINT
        },
        inner.width(),
    );
    let title_pos = egui::pos2(inner.left(), badge.bottom() + 12.0);
    let description_pos = title_pos + egui::vec2(0.0, title.size().y + 4.0);
    painter.galley(title_pos, title, title_color);
    painter.galley(description_pos, description, theme::TEXT_MUTED);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl eframe::App for LastLightApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl LastLightApp {
    /// Draws the whole window. Separate from `eframe::App::ui` so tests can
    /// render it without a real window.
    fn show(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| self.header(ui));

        // Apps draw edge to edge (their own panels bring margins); the grid gets padding.
        let margin = match self.open {
            Some(_) => egui::Margin::ZERO,
            None => egui::Margin::symmetric(28, 20),
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(margin))
            .show(ui, |ui| match self.open {
                Some(i) => self.apps[i].ui(ui),
                None => self.card_grid(ui),
            });
    }
}

#[cfg(test)]
mod screenshots {
    use super::*;

    /// Renders each main screen with sample data to PNG files, for checking
    /// the design and for the README. Writes to `target/screenshots/`.
    ///
    /// `cargo test render_screenshots -- --ignored`
    #[test]
    #[ignore = "renders images with the GPU; run by hand"]
    fn render_screenshots() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/screenshots");
        std::fs::create_dir_all(&out).unwrap();

        for (file, open) in [
            ("grid", None),
            ("text-editor", Some(0)),
            ("ai-chat", Some(1)),
        ] {
            let notes = tempfile::tempdir().unwrap();
            let mut app = LastLightApp {
                apps: apps::demo(notes.path()),
                open,
            };
            // kittest wraps the UI in an 8 px margin; render that much larger
            // and crop it off, so the image matches a real 1024x720 window.
            let mut harness = egui_kittest::Harness::builder()
                .with_size(egui::vec2(1024.0 + 16.0, 720.0 + 16.0))
                .with_context_setup(theme::apply)
                // egui_commonmark's task-list checkboxes have no accessible
                // name, which we can't fix from here.
                .with_accessibility_check(false)
                .build_ui(|ui| app.show(ui));
            harness.run_steps(4);
            let image = harness.render().expect("render screen");
            let (width, height) = (image.width() - 16, image.height() - 16);
            image::imageops::crop_imm(&image, 8, 8, width, height)
                .to_image()
                .save(out.join(format!("{file}.png")))
                .unwrap();
        }
    }
}
