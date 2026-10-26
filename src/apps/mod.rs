mod ai_chat;
mod text_editor;
mod widgets;

use eframe::egui;

/// A mini app shown as a card on the main screen.
pub trait MiniApp {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn ui(&mut self, ui: &mut egui::Ui);
    /// Called when the user leaves the app for the card grid, e.g. to flush unsaved work.
    fn on_close(&mut self) {}
}

/// Stand-in for apps that aren't built yet.
pub struct Placeholder {
    name: &'static str,
    description: &'static str,
}

impl Placeholder {
    pub fn new(name: &'static str, description: &'static str) -> Self {
        Self { name, description }
    }
}

impl MiniApp for Placeholder {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        self.description
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            ui.heading(self.name);
            ui.label("Coming soon.");
        });
    }
}

pub fn all() -> Vec<Box<dyn MiniApp>> {
    vec![
        Box::new(text_editor::TextEditor::new()),
        Box::new(ai_chat::AiChat::new()),
        Box::new(Placeholder::new("Journal", "Keep a dated log of each day.")),
        Box::new(Placeholder::new(
            "Calculator",
            "Numbers and unit conversions.",
        )),
        Box::new(Placeholder::new(
            "Survival Guide",
            "Offline first aid and survival reference.",
        )),
        Box::new(Placeholder::new("Supplies", "Track food, water and gear.")),
    ]
}
