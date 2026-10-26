use eframe::egui;

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
        Box::new(|_cc| Ok(Box::new(LastLightApp::default()))),
    )
}

#[derive(Default)]
struct LastLightApp {}

impl eframe::App for LastLightApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("header").show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading("Last Light");
            });
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ui, |_ui| {});
    }
}
