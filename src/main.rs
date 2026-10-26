mod apps;

use apps::MiniApp;
use eframe::egui;

const CARD_SIZE: egui::Vec2 = egui::vec2(220.0, 120.0);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Last Light")
            .with_inner_size([1024.0, 720.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Last Light",
        options,
        Box::new(|_cc| Ok(Box::new(LastLightApp::new()))),
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

    fn card_grid(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
                for (i, app) in self.apps.iter().enumerate() {
                    if card(ui, app.name(), app.description()).clicked() {
                        self.open = Some(i);
                    }
                }
            });
        });
    }
}

fn card(ui: &mut egui::Ui, title: &str, description: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE, egui::Sense::click());
    let visuals = ui.style().interact(&response);

    ui.painter().rect(
        rect,
        8.0,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );

    // Text is painted rather than added as widgets, so it can't steal the card's clicks.
    let inner = rect.shrink(16.0);
    let painter = ui.painter();
    let title = painter.layout(
        title.to_owned(),
        egui::TextStyle::Heading.resolve(ui.style()),
        ui.visuals().strong_text_color(),
        inner.width(),
    );
    let description = painter.layout(
        description.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        visuals.text_color(),
        inner.width(),
    );
    let description_pos = inner.left_top() + egui::vec2(0.0, title.size().y + 6.0);
    painter.galley(inner.left_top(), title, visuals.text_color());
    painter.galley(description_pos, description, visuals.text_color());

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

impl eframe::App for LastLightApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if self.open.is_some() && ui.button("< Back").clicked() {
                    self.open = None;
                }
                ui.heading("Last Light");
            });
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ui, |ui| match self.open {
            Some(i) => self.apps[i].ui(ui),
            None => self.card_grid(ui),
        });
    }
}
