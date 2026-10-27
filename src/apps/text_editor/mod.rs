//! Text Editor: Markdown notes in plain files, organized one folder deep. A
//! toggleable sidebar lists folders and the selected folder's notes; the
//! editor shows the source, a rendered preview, or both side by side.

mod store;

use std::time::{Duration, Instant};

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use super::MiniApp;
use super::widgets::{InputOutcome, RowAction, inline_input, list_row, section_header};
use crate::theme;
use store::{DEFAULT_FOLDER, NotesStore};

/// How long typing must pause before the note is saved.
const AUTOSAVE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Split,
    Edit,
    View,
}

impl Mode {
    const ALL: [Mode; 3] = [Mode::Split, Mode::Edit, Mode::View];

    fn label(self) -> &'static str {
        match self {
            Mode::Split => "Split",
            Mode::Edit => "Edit",
            Mode::View => "View",
        }
    }
}

struct OpenNote {
    folder: String,
    name: String,
    content: String,
    /// When the content last changed without being saved; `None` when saved.
    unsaved_since: Option<Instant>,
}

/// The inline text field currently shown in the sidebar, if any.
enum Editing {
    NewFolder(String),
    NewNote(String),
    RenameFolder { old: String, text: String },
    RenameNote { old: String, text: String },
}

/// A deletion waiting for the user to confirm it.
enum Confirm {
    DeleteFolder { name: String, notes: usize },
    DeleteNote { folder: String, name: String },
}

enum SidebarAction {
    SelectFolder(String),
    OpenNote(String),
    StartEditing(Editing),
    FinishEditing,
    CancelEditing,
    AskDeleteFolder(String),
    AskDeleteNote(String),
}

pub struct TextEditor {
    store: NotesStore,
    folders: Vec<String>,
    /// The selected folder, whose notes are listed in the sidebar.
    folder: Option<String>,
    /// Notes in the selected folder.
    notes: Vec<String>,
    open: Option<OpenNote>,
    mode: Mode,
    show_sidebar: bool,
    editing: Option<Editing>,
    confirm: Option<Confirm>,
    notice: Option<String>,
    preview_cache: CommonMarkCache,
}

impl TextEditor {
    pub fn new() -> Self {
        let (store, warning) = NotesStore::open();
        Self::with_store(store, warning)
    }

    fn with_store(store: NotesStore, warning: Option<String>) -> Self {
        let mut editor = Self {
            store,
            folders: Vec::new(),
            folder: None,
            notes: Vec::new(),
            open: None,
            mode: Mode::Split,
            show_sidebar: true,
            editing: None,
            confirm: None,
            notice: warning,
            preview_cache: CommonMarkCache::default(),
        };
        editor.refresh_folders();
        let start = editor
            .folders
            .iter()
            .find(|f| f.as_str() == DEFAULT_FOLDER)
            .or(editor.folders.first())
            .cloned();
        if let Some(folder) = start {
            editor.select_folder(folder);
        }
        editor
    }

    /// Shows a file error to the user instead of failing silently.
    fn check<T>(&mut self, result: Result<T, String>) -> Option<T> {
        result.map_err(|e| self.notice = Some(e)).ok()
    }

    fn refresh_folders(&mut self) {
        if let Some(folders) = self.check(self.store.folders()) {
            self.folders = folders;
        }
    }

    fn refresh_notes(&mut self) {
        let notes = match &self.folder {
            Some(folder) => self.store.notes(folder),
            None => Ok(Vec::new()),
        };
        self.notes = self.check(notes).unwrap_or_default();
    }

    fn select_folder(&mut self, folder: String) {
        self.folder = Some(folder);
        self.refresh_notes();
    }

    fn open_note(&mut self, name: String) {
        let Some(folder) = self.folder.clone() else {
            return;
        };
        if self
            .open
            .as_ref()
            .is_some_and(|n| n.folder == folder && n.name == name)
        {
            return;
        }
        self.save();
        if let Some(content) = self.check(self.store.read_note(&folder, &name)) {
            self.open = Some(OpenNote {
                folder,
                name,
                content,
                unsaved_since: None,
            });
        }
    }

    /// Writes the open note to disk if it has unsaved changes.
    fn save(&mut self) {
        let Some(note) = &self.open else { return };
        if note.unsaved_since.is_none() {
            return;
        }
        let result = self
            .store
            .write_note(&note.folder, &note.name, &note.content);
        if self.check(result).is_some()
            && let Some(note) = &mut self.open
        {
            note.unsaved_since = None;
        }
    }

    /// Saves once typing has paused for `AUTOSAVE_DELAY`, and makes sure a
    /// frame runs then even if there's no input.
    fn autosave(&mut self, ctx: &egui::Context) {
        let Some(since) = self.open.as_ref().and_then(|n| n.unsaved_since) else {
            return;
        };
        let elapsed = since.elapsed();
        if elapsed >= AUTOSAVE_DELAY {
            self.save();
        } else {
            ctx.request_repaint_after(AUTOSAVE_DELAY - elapsed);
        }
    }

    fn apply(&mut self, action: SidebarAction) {
        match action {
            SidebarAction::SelectFolder(folder) => self.select_folder(folder),
            SidebarAction::OpenNote(name) => self.open_note(name),
            SidebarAction::StartEditing(editing) => {
                if matches!(editing, Editing::NewNote(_)) && self.folder.is_none() {
                    return;
                }
                self.editing = Some(editing);
            }
            SidebarAction::FinishEditing => self.finish_editing(),
            SidebarAction::CancelEditing => self.editing = None,
            SidebarAction::AskDeleteFolder(name) => {
                let notes = self.store.notes(&name).map_or(0, |notes| notes.len());
                self.confirm = Some(Confirm::DeleteFolder { name, notes });
            }
            SidebarAction::AskDeleteNote(name) => {
                if let Some(folder) = self.folder.clone() {
                    self.confirm = Some(Confirm::DeleteNote { folder, name });
                }
            }
        }
    }

    fn finish_editing(&mut self) {
        let Some(editing) = self.editing.take() else {
            return;
        };
        match editing {
            Editing::NewFolder(text) => {
                // Clicking away from an empty field just cancels.
                if text.trim().is_empty() {
                    return;
                }
                if let Some(name) = self.check(self.store.create_folder(&text)) {
                    self.refresh_folders();
                    self.select_folder(name);
                }
            }
            Editing::NewNote(text) => {
                let Some(folder) = self.folder.clone() else {
                    return;
                };
                if text.trim().is_empty() {
                    return;
                }
                if let Some(name) = self.check(self.store.create_note(&folder, &text)) {
                    self.refresh_notes();
                    self.open_note(name);
                }
            }
            Editing::RenameFolder { old, text } => {
                // Flush first: the pending save targets the old path.
                self.save();
                let Some(new) = self.check(self.store.rename_folder(&old, &text)) else {
                    return;
                };
                if self.folder.as_deref() == Some(old.as_str()) {
                    self.folder = Some(new.clone());
                }
                if let Some(note) = self.open.as_mut().filter(|n| n.folder == old) {
                    note.folder = new;
                }
                self.refresh_folders();
                self.refresh_notes();
            }
            Editing::RenameNote { old, text } => {
                let Some(folder) = self.folder.clone() else {
                    return;
                };
                self.save();
                let Some(new) = self.check(self.store.rename_note(&folder, &old, &text)) else {
                    return;
                };
                if let Some(note) = self
                    .open
                    .as_mut()
                    .filter(|n| n.folder == folder && n.name == old)
                {
                    note.name = new;
                }
                self.refresh_notes();
            }
        }
    }

    fn delete_confirmed(&mut self) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        match confirm {
            Confirm::DeleteFolder { name, .. } => {
                if self.check(self.store.delete_folder(&name)).is_none() {
                    return;
                }
                if self.open.as_ref().is_some_and(|n| n.folder == name) {
                    self.open = None;
                }
                self.refresh_folders();
                if self.folder.as_deref() == Some(name.as_str()) {
                    self.folder = None;
                    if let Some(first) = self.folders.first().cloned() {
                        self.select_folder(first);
                    } else {
                        self.notes.clear();
                    }
                }
            }
            Confirm::DeleteNote { folder, name } => {
                if self.check(self.store.delete_note(&folder, &name)).is_none() {
                    return;
                }
                if self
                    .open
                    .as_ref()
                    .is_some_and(|n| n.folder == folder && n.name == name)
                {
                    self.open = None;
                }
                self.refresh_notes();
            }
        }
    }

    fn folders_panel(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        if section_header(ui, "Folders", "New folder", true) {
            action = Some(SidebarAction::StartEditing(Editing::NewFolder(
                String::new(),
            )));
        }
        if let Some(Editing::NewFolder(text)) = &mut self.editing
            && let Some(outcome) = inline_input(ui, text, "Folder name")
        {
            action = Some(finish_or_cancel(outcome));
        }

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for folder in &self.folders {
                    let rename = match &mut self.editing {
                        Some(Editing::RenameFolder { old, text }) if old == folder => Some(text),
                        _ => None,
                    };
                    let selected = self.folder.as_ref() == Some(folder);
                    if let Some(row) = list_row(ui, folder, selected, rename) {
                        action = Some(match row {
                            RowAction::Open => SidebarAction::SelectFolder(folder.clone()),
                            RowAction::StartRename => {
                                SidebarAction::StartEditing(Editing::RenameFolder {
                                    old: folder.clone(),
                                    text: folder.clone(),
                                })
                            }
                            RowAction::FinishRename => SidebarAction::FinishEditing,
                            RowAction::CancelRename => SidebarAction::CancelEditing,
                            RowAction::Delete => SidebarAction::AskDeleteFolder(folder.clone()),
                        });
                    }
                }
                if self.folders.is_empty() {
                    ui.weak("No folders yet.");
                }
            });
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn notes_panel(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        let title = self.folder.clone().unwrap_or_else(|| "Notes".to_owned());
        if section_header(ui, &title, "New note", self.folder.is_some()) {
            action = Some(SidebarAction::StartEditing(Editing::NewNote(String::new())));
        }
        if let Some(Editing::NewNote(text)) = &mut self.editing
            && let Some(outcome) = inline_input(ui, text, "Note name")
        {
            action = Some(finish_or_cancel(outcome));
        }

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for note in &self.notes {
                    let rename = match &mut self.editing {
                        Some(Editing::RenameNote { old, text }) if old == note => Some(text),
                        _ => None,
                    };
                    let selected = self.open.as_ref().is_some_and(|n| {
                        Some(&n.folder) == self.folder.as_ref() && &n.name == note
                    });
                    if let Some(row) = list_row(ui, note, selected, rename) {
                        action = Some(match row {
                            RowAction::Open => SidebarAction::OpenNote(note.clone()),
                            RowAction::StartRename => {
                                SidebarAction::StartEditing(Editing::RenameNote {
                                    old: note.clone(),
                                    text: note.clone(),
                                })
                            }
                            RowAction::FinishRename => SidebarAction::FinishEditing,
                            RowAction::CancelRename => SidebarAction::CancelEditing,
                            RowAction::Delete => SidebarAction::AskDeleteNote(note.clone()),
                        });
                    }
                }
                if self.folder.is_none() {
                    ui.weak("Select a folder.");
                } else if self.notes.is_empty() {
                    ui.weak("No notes yet.");
                }
            });
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let sidebar = if self.show_sidebar {
                "Hide sidebar"
            } else {
                "Show sidebar"
            };
            if ui.add(theme::quiet_button(sidebar)).clicked() {
                self.show_sidebar = !self.show_sidebar;
            }
            ui.separator();

            // Right to left so the mode switch keeps its size and the note
            // title is truncated to whatever space is left.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let modes = Mode::ALL.map(|mode| (mode, mode.label()));
                if let Some(mode) = theme::segmented(ui, self.open.is_some(), self.mode, &modes) {
                    self.mode = mode;
                }
                if let Some(note) = &self.open {
                    let (status, color) = match note.unsaved_since {
                        Some(_) => ("Saving...", theme::ACCENT),
                        None => ("Saved", theme::TEXT_FAINT),
                    };
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(status).small().color(color));
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let mut path = egui::text::LayoutJob::default();
                        let font = egui::FontId::proportional(14.5);
                        let part = |color| egui::TextFormat::simple(font.clone(), color);
                        path.append(&format!("{} / ", note.folder), 0.0, part(theme::TEXT_FAINT));
                        path.append(&note.name, 0.0, part(theme::TEXT_STRONG));
                        ui.add(egui::Label::new(path).truncate());
                    });
                }
            });
        });
        if let Some(notice) = self.notice.clone() {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add(theme::quiet_button("x"))
                    .on_hover_text("Dismiss")
                    .clicked()
                {
                    self.notice = None;
                }
                ui.add(egui::Label::new(egui::RichText::new(notice).color(theme::WARN)).wrap());
            });
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let Some(note) = &mut self.open else {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                theme::lamp(ui, 36.0);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("No note open")
                        .size(17.0)
                        .color(theme::TEXT),
                );
                ui.label(
                    egui::RichText::new("Pick a note from the sidebar, or create one with +.")
                        .color(theme::TEXT_MUTED),
                );
            });
            return;
        };
        let changed = match self.mode {
            Mode::Edit => edit_pane(ui, note),
            Mode::View => preview_pane(ui, &mut self.preview_cache, note),
            Mode::Split => {
                ui.spacing_mut().item_spacing.x = 16.0;
                ui.columns(2, |columns| {
                    let [left, right] = columns else {
                        unreachable!("two columns")
                    };
                    let edited = edit_pane(left, note);
                    preview_pane(right, &mut self.preview_cache, note) || edited
                })
            }
        };
        if changed {
            note.unsaved_since = Some(Instant::now());
        }
    }

    fn confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(confirm) = &self.confirm else { return };
        let (title, body) = match confirm {
            Confirm::DeleteFolder { name, notes } => (
                format!("Delete folder \"{name}\"?"),
                match notes {
                    0 => "The folder is empty.".to_owned(),
                    1 => "Its 1 note will be deleted too. This can't be undone.".to_owned(),
                    n => format!("Its {n} notes will be deleted too. This can't be undone."),
                },
            ),
            Confirm::DeleteNote { name, .. } => (
                format!("Delete note \"{name}\"?"),
                "This can't be undone.".to_owned(),
            ),
        };

        let mut decision = None;
        let modal = egui::Modal::new(egui::Id::unique("text_editor_confirm")).show(ctx, |ui| {
            ui.set_max_width(360.0);
            ui.heading(title);
            ui.add_space(4.0);
            ui.label(body);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                let delete = egui::RichText::new("Delete")
                    .strong()
                    .color(theme::TEXT_STRONG);
                if ui
                    .add(egui::Button::new(delete).fill(theme::DANGER))
                    .clicked()
                {
                    decision = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    decision = Some(false);
                }
            });
        });
        if decision.is_none() && modal.should_close() {
            decision = Some(false);
        }
        match decision {
            Some(true) => self.delete_confirmed(),
            Some(false) => self.confirm = None,
            None => {}
        }
    }
}

impl MiniApp for TextEditor {
    fn name(&self) -> &str {
        "Text Editor"
    }

    fn description(&self) -> &str {
        "Markdown notes, organized in folders."
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        if ui.input_mut(|i| {
            i.consume_shortcut(&egui::KeyboardShortcut::new(
                egui::Modifiers::COMMAND,
                egui::Key::S,
            ))
        }) {
            self.save();
        }
        self.autosave(ui.ctx());

        if self.show_sidebar {
            // The folders column sits a shade darker than the notes column,
            // so the two read as levels rather than one wide list.
            let column = |fill| {
                egui::Frame::new()
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric(10, 4))
            };
            egui::Panel::left("notes_folders")
                .resizable(true)
                .default_size(150.0)
                .size_range(110.0..=260.0)
                .frame(column(theme::BG))
                .show(ui, |ui| self.folders_panel(ui));
            egui::Panel::left("notes_list")
                .resizable(true)
                .default_size(200.0)
                .size_range(140.0..=360.0)
                .frame(column(theme::PANEL))
                .show(ui, |ui| self.notes_panel(ui));
        }
        egui::Panel::top("notes_toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ui, |ui| self.editor(ui));

        self.confirm_dialog(ui.ctx());
    }

    fn on_close(&mut self) {
        self.save();
    }
}

impl Drop for TextEditor {
    /// Don't lose the last keystrokes when the window closes mid-autosave.
    fn drop(&mut self) {
        self.save();
    }
}

fn finish_or_cancel(outcome: InputOutcome) -> SidebarAction {
    match outcome {
        InputOutcome::Commit => SidebarAction::FinishEditing,
        InputOutcome::Cancel => SidebarAction::CancelEditing,
    }
}

/// The Markdown source. Returns true if the text changed.
fn edit_pane(ui: &mut egui::Ui, note: &mut OpenNote) -> bool {
    egui::ScrollArea::vertical()
        .id_salt("note_source")
        .auto_shrink(false)
        .show(ui, |ui| {
            // Frameless: the source reads as a writing surface, not a form field.
            ui.add(
                egui::TextEdit::multiline(&mut note.content)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Start writing Markdown...")
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY)
                    .min_size(ui.available_size())
                    .lock_focus(true),
            )
            .changed()
        })
        .inner
}

/// The rendered Markdown. Ticking a task-list checkbox edits the source, so
/// this returns true if the text changed.
fn preview_pane(ui: &mut egui::Ui, cache: &mut CommonMarkCache, note: &mut OpenNote) -> bool {
    // The rendered note sits on its own raised page.
    egui::Frame::new()
        .fill(theme::PANEL)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("note_preview")
                .auto_shrink(false)
                .show(ui, |ui| {
                    CommonMarkViewer::new()
                        .show_mut(ui, cache, &mut note.content)
                        .response
                        .changed()
                })
                .inner
        })
        .inner
}

#[cfg(test)]
impl TextEditor {
    /// An editor over sample notes in `root`, with one note open in Split mode.
    pub(super) fn demo(root: &std::path::Path) -> Self {
        let store = NotesStore::at(root.join("notes"));
        store.init().unwrap();
        let notes = [
            ("Water", "Purifying river water", DEMO_NOTE),
            ("Water", "Rainwater collection", "# Rainwater\n"),
            ("Water", "Wells nearby", "# Wells\n"),
            ("Food", "Pantry", "# Pantry\n"),
            ("Food", "Foraging", "# Foraging\n"),
            ("Shelter", "Repairs", "# Repairs\n"),
            (DEFAULT_FOLDER, "Ideas", "# Ideas\n"),
        ];
        for (folder, name, content) in notes {
            if !store.folders().unwrap().iter().any(|f| f == folder) {
                store.create_folder(folder).unwrap();
            }
            store.create_note(folder, name).unwrap();
            store.write_note(folder, name, content).unwrap();
        }
        let mut editor = Self::with_store(store, None);
        editor.select_folder("Water".to_owned());
        editor.open_note("Purifying river water".to_owned());
        editor
    }
}

#[cfg(test)]
const DEMO_NOTE: &str = "# Purifying river water

Collect from **moving water**, upstream of any camp.

## Steps

- [x] Let sediment settle for an hour
- [x] Pour through a cloth into a clean pot
- [ ] Bring to a rolling boil for `1 minute`
- [ ] Store covered, away from sunlight

## Fuel per litre

| Method | Time | Fuel |
|---|---|---|
| Boiling | 1 min | Wood |
| Sun (SODIS) | 6 h | None |

> Above 2,000 m, boil for 3 minutes.
";

#[cfg(test)]
mod tests {
    use super::*;

    const NOTE: &str = "# Water\n\n- [ ] Boil for one minute\n- [x] Find a source\n\n| Item | Litres |\n|---|---|\n| Bottle | 1.5 |\n\n`code` and **bold** text.";

    fn editor_with_long_names() -> (tempfile::TempDir, TextEditor) {
        let dir = tempfile::tempdir().unwrap();
        let store = NotesStore::at(dir.path().join("notes"));
        store.init().unwrap();
        let folder = store.create_folder(&"Long folder name ".repeat(5)).unwrap();
        let note = store
            .create_note(&folder, &"Long note name ".repeat(6))
            .unwrap();
        store.write_note(&folder, &note, NOTE).unwrap();

        let mut editor = TextEditor::with_store(store, None);
        editor.select_folder(folder);
        editor.open_note(note);
        assert!(editor.open.is_some());
        (dir, editor)
    }

    /// Renders a few frames in a 1024x720 window and returns the final widths
    /// of the folders and notes panels (`None` while the sidebar is hidden).
    fn render(editor: &mut TextEditor) -> (Option<f32>, Option<f32>) {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1024.0, 720.0),
            )),
            ..Default::default()
        };
        let mut widths = (None, None);
        for _ in 0..10 {
            let output = ctx.run_ui(input.clone(), |ui| {
                editor.ui(ui);
                // A panel's id is its parent's scope id salted with its name.
                let width = |name: &str| {
                    let id = ui.scope_id().with_salt(egui::IdSalt::new(name));
                    egui::containers::panel::PanelState::load(ui.ctx(), id).map(|s| s.size().x)
                };
                widths = (width("notes_folders"), width("notes_list"));
            });
            output.drop_without_applying_deltas();
        }
        widths
    }

    #[test]
    fn long_names_do_not_widen_the_sidebar_in_any_mode() {
        let (_dir, mut editor) = editor_with_long_names();
        for mode in Mode::ALL {
            editor.mode = mode;
            let (folders, notes) = render(&mut editor);
            assert_eq!(folders, Some(150.0), "{} folders panel", mode.label());
            assert_eq!(notes, Some(200.0), "{} notes panel", mode.label());
        }
        editor.show_sidebar = false;
        render(&mut editor); // the editor alone must render too
    }

    #[test]
    fn edits_are_saved_on_close() {
        let (_dir, mut editor) = editor_with_long_names();
        let note = editor.open.as_mut().unwrap();
        note.content.push_str("\nMore.");
        note.unsaved_since = Some(Instant::now());
        let (folder, name) = (note.folder.clone(), note.name.clone());
        editor.on_close();
        assert!(
            editor
                .store
                .read_note(&folder, &name)
                .unwrap()
                .ends_with("More.")
        );
    }

    #[test]
    fn renaming_the_open_note_keeps_it_open_and_saved() {
        let (_dir, mut editor) = editor_with_long_names();
        let old = editor.open.as_ref().unwrap().name.clone();
        editor.open.as_mut().unwrap().content.push_str("\nUnsaved.");
        editor.open.as_mut().unwrap().unsaved_since = Some(Instant::now());
        editor.editing = Some(Editing::RenameNote {
            old,
            text: "Water".to_owned(),
        });
        editor.finish_editing();
        let note = editor.open.as_ref().unwrap();
        assert_eq!(note.name, "Water");
        assert!(
            editor
                .store
                .read_note(&note.folder, "Water")
                .unwrap()
                .ends_with("Unsaved.")
        );
        assert_eq!(editor.notes, ["Water"]);
    }
}
