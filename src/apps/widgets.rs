//! Small widgets shared by mini apps.

use eframe::egui;

use crate::theme;

const ROW_HEIGHT: f32 = 28.0;

/// A sidebar section heading (e.g. "FOLDERS") with a "+" button on the right.
/// Returns true when "+" is clicked. Lays out right to left so a long heading
/// is truncated instead of widening the panel.
pub fn section_header(ui: &mut egui::Ui, title: &str, add_hint: &str, can_add: bool) -> bool {
    ui.add_space(10.0);
    let mut clicked = false;
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 22.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            clicked = ui
                .add_enabled(can_add, theme::quiet_button("+"))
                .on_hover_text(add_hint)
                .clicked();
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(theme::section_label(title)).truncate());
            });
        },
    );
    ui.add_space(4.0);
    clicked
}

/// What the user did with a [`list_row`] this frame.
pub enum RowAction {
    Open,
    StartRename,
    FinishRename,
    CancelRename,
    Delete,
}

/// How an [`inline_input`] was closed.
pub enum InputOutcome {
    Commit,
    Cancel,
}

/// A single-line text field that takes focus when shown. Enter or clicking
/// away commits; Escape cancels.
pub fn inline_input(ui: &mut egui::Ui, text: &mut String, hint: &str) -> Option<InputOutcome> {
    let edit = ui.add(
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    );
    if edit.lost_focus() {
        let cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));
        return Some(if cancelled {
            InputOutcome::Cancel
        } else {
            InputOutcome::Commit
        });
    }
    if !edit.has_focus() {
        edit.request_focus();
    }
    None
}

/// One row in a sidebar list: a selectable, truncated title plus a "..." menu
/// with Rename and Delete (also on right-click); double-click renames. While
/// `rename` is `Some`, shows a text field editing that text instead.
pub fn list_row(
    ui: &mut egui::Ui,
    title: &str,
    selected: bool,
    rename: Option<&mut String>,
) -> Option<RowAction> {
    if let Some(text) = rename {
        return inline_input(ui, text, "").map(|outcome| match outcome {
            InputOutcome::Commit => RowAction::FinishRename,
            InputOutcome::Cancel => RowAction::CancelRename,
        });
    }

    // Lay out right to left: the menu button takes its natural width first and
    // the title gets exactly what's left, so a long title is truncated instead
    // of overflowing the row (which would make the panel grow).
    let mut action = None;
    let row = egui::Layout::right_to_left(egui::Align::Center);
    ui.allocate_ui_with_layout(egui::vec2(ui.available_width(), ROW_HEIGHT), row, |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        egui::containers::menu::MenuButton::from_button(theme::quiet_button("..."))
            .ui(ui, |ui| action = row_menu(ui).or(action.take()));

        let title_width = ui.available_width();
        let color = if selected { theme::ACCENT } else { theme::TEXT };
        let title_button = ui
            .scope_builder(
                egui::UiBuilder::new().layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    ui.set_max_width(title_width);
                    // The grow atom takes the spare width, so the name sits on the left.
                    let text = egui::RichText::new(title).color(color);
                    ui.add(
                        egui::Button::selectable(selected, (text, egui::Atom::grow()))
                            .frame_when_inactive(selected)
                            .truncate()
                            .min_size(egui::vec2(title_width, ROW_HEIGHT)),
                    )
                },
            )
            .inner
            .on_hover_text(title);
        if title_button.clicked() {
            action = Some(RowAction::Open);
        }
        if title_button.double_clicked() {
            action = Some(RowAction::StartRename);
        }
        title_button.context_menu(|ui| action = row_menu(ui).or(action.take()));
    });
    action
}

fn row_menu(ui: &mut egui::Ui) -> Option<RowAction> {
    if ui.button("Rename").clicked() {
        return Some(RowAction::StartRename);
    }
    if ui.button("Delete").clicked() {
        return Some(RowAction::Delete);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders a resizable side panel holding one row for several frames and
    /// returns its width each frame.
    fn panel_widths(title: &str) -> Vec<f32> {
        let ctx = egui::Context::default();
        let mut widths = Vec::new();
        for _ in 0..20 {
            let output = ctx.run_ui(Default::default(), |ui| {
                let panel = egui::Panel::left("list")
                    .resizable(true)
                    .default_size(220.0)
                    .size_range(160.0..=400.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink(false)
                            .show(ui, |ui| list_row(ui, title, true, None));
                    });
                widths.push(panel.response.rect.width());
            });
            output.drop_without_applying_deltas();
        }
        widths
    }

    #[test]
    fn long_titles_do_not_widen_the_panel() {
        let short = panel_widths("Water");
        let long = panel_widths(&"A very long title that keeps going ".repeat(10));
        assert_eq!(short.last(), long.last());
        assert!(long.iter().all(|w| *w <= 400.0));
    }
}
