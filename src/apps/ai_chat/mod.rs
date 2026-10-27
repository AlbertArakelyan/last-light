//! AI Chat: conversations with a local LLM, saved in SQLite. A side panel
//! lists past chats, which are auto-named and can be renamed or deleted.

mod store;
mod worker;

use eframe::egui;

use super::MiniApp;
use super::widgets::{RowAction, list_row, section_header};
use crate::theme;
use store::{ChatSummary, Store};
use worker::{Event, Request, Worker};

const TITLE_MAX_CHARS: usize = 48;

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }

    pub fn parse(role: &str) -> Option<Self> {
        match role {
            "system" => Some(Role::System),
            "user" => Some(Role::User),
            "assistant" => Some(Role::Assistant),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

enum ModelStatus {
    Loading,
    Ready,
    Failed(String),
}

/// A reply being streamed from the model.
struct Streaming {
    chat_id: i64,
    text: String,
}

enum ChatAction {
    Open(i64),
    StartRename(i64),
    FinishRename,
    CancelRename,
    Delete(i64),
}

pub struct AiChat {
    store: Store,
    worker: Option<Worker>,
    model: ModelStatus,
    chats: Vec<ChatSummary>,
    /// The open chat; `None` is a new chat that isn't saved until its first message.
    current: Option<i64>,
    messages: Vec<Message>,
    streaming: Option<Streaming>,
    /// The chat being renamed and its edited title.
    renaming: Option<(i64, String)>,
    notice: Option<String>,
    input: String,
}

impl AiChat {
    pub fn new() -> Self {
        let (store, warning) = Store::open();
        let mut app = Self {
            store,
            worker: None,
            model: ModelStatus::Loading,
            chats: Vec::new(),
            current: None,
            messages: Vec::new(),
            streaming: None,
            renaming: None,
            notice: warning,
            input: String::new(),
        };
        app.refresh_chats();
        app
    }

    /// Shows a database error to the user instead of failing silently.
    fn check<T>(&mut self, result: rusqlite::Result<T>) -> Option<T> {
        result
            .map_err(|e| self.notice = Some(format!("Database error: {e}")))
            .ok()
    }

    fn refresh_chats(&mut self) {
        if let Some(chats) = self.check(self.store.chats()) {
            self.chats = chats;
        }
    }

    fn new_chat(&mut self) {
        self.current = None;
        self.messages.clear();
    }

    fn apply(&mut self, action: ChatAction) {
        match action {
            ChatAction::Open(id) => {
                self.current = Some(id);
                self.messages = self.check(self.store.messages(id)).unwrap_or_default();
            }
            ChatAction::StartRename(id) => {
                let title = self
                    .chats
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.title.clone());
                self.renaming = title.map(|title| (id, title));
            }
            ChatAction::FinishRename => {
                if let Some((id, title)) = self.renaming.take() {
                    let title = truncate_title(&title);
                    if !title.is_empty() {
                        self.check(self.store.rename_chat(id, &title));
                        self.refresh_chats();
                    }
                }
            }
            ChatAction::CancelRename => self.renaming = None,
            ChatAction::Delete(id) => {
                self.check(self.store.delete_chat(id));
                if self.current == Some(id) {
                    self.new_chat();
                }
                self.refresh_chats();
            }
        }
    }

    fn poll_events(&mut self) {
        let Some(worker) = &self.worker else { return };
        let events: Vec<Event> = std::iter::from_fn(|| worker.poll()).collect();
        for event in events {
            self.handle(event);
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Ready => self.model = ModelStatus::Ready,
            Event::LoadFailed(err) => self.model = ModelStatus::Failed(err),
            Event::Token { chat_id, piece } => {
                if let Some(streaming) = &mut self.streaming
                    && streaming.chat_id == chat_id
                {
                    streaming.text.push_str(&piece);
                }
            }
            Event::ReplyDone { chat_id } => {
                let Some(streaming) = self.streaming.take() else {
                    return;
                };
                // The chat may have been deleted while the reply was streaming.
                if !self.chats.iter().any(|c| c.id == chat_id) {
                    return;
                }
                let reply = Message {
                    role: Role::Assistant,
                    content: streaming.text.trim().to_owned(),
                };
                self.check(self.store.add_message(chat_id, &reply));
                if self.current == Some(chat_id) {
                    self.messages.push(reply);
                }
                self.refresh_chats();
            }
            Event::ReplyFailed { error } => {
                self.streaming = None;
                self.notice = Some(format!("The AI couldn't reply: {error}"));
            }
            Event::Title { chat_id, title } => {
                self.check(self.store.set_auto_title(chat_id, &title));
                self.refresh_chats();
            }
        }
    }

    fn send(&mut self) {
        let text = self.input.trim().to_owned();
        if text.is_empty() || self.streaming.is_some() || !matches!(self.model, ModelStatus::Ready)
        {
            return;
        }

        // A new chat is saved on its first message, titled after it until the AI names it.
        let chat_id = match self.current {
            Some(id) => id,
            None => {
                let Some(id) = self.check(self.store.create_chat(&truncate_title(&text))) else {
                    return;
                };
                self.current = Some(id);
                id
            }
        };
        let is_first = self.messages.is_empty();
        let message = Message {
            role: Role::User,
            content: text.clone(),
        };
        if self
            .check(self.store.add_message(chat_id, &message))
            .is_none()
        {
            return;
        }
        self.messages.push(message);
        self.input.clear();
        self.notice = None;
        self.refresh_chats();

        let Some(worker) = &self.worker else { return };
        let mut sent = worker.send(Request::Reply {
            chat_id,
            history: self.messages.clone(),
        });
        if is_first {
            sent &= worker.send(Request::Title {
                chat_id,
                first_message: text,
            });
        }
        if sent {
            self.streaming = Some(Streaming {
                chat_id,
                text: String::new(),
            });
        } else {
            self.model = ModelStatus::Failed("The AI stopped unexpectedly.".to_owned());
        }
    }

    fn chat_list(&mut self, ui: &mut egui::Ui) {
        if section_header(ui, "Chats", "New chat", true) {
            self.new_chat();
        }

        let mut action = None;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for chat in &self.chats {
                    let selected = self.current == Some(chat.id);
                    if let Some(a) = chat_row(ui, chat, selected, &mut self.renaming) {
                        action = Some(a);
                    }
                }
                if self.chats.is_empty() {
                    ui.label(egui::RichText::new("No chats yet.").color(theme::TEXT_FAINT));
                }
            });
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn input_area(&mut self, ui: &mut egui::Ui) {
        reading_column(ui, |ui| {
            if let Some(notice) = self.notice.clone() {
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
                ui.add_space(4.0);
            }
            match &self.model {
                ModelStatus::Loading => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(egui::RichText::new("Loading model...").color(theme::TEXT_MUTED));
                    });
                }
                ModelStatus::Failed(err) => {
                    ui.label(egui::RichText::new(err).color(theme::ERROR));
                }
                ModelStatus::Ready => self.input_bar(ui),
            }
        });
    }

    fn input_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new()
            .fill(theme::RAISED)
            .stroke(egui::Stroke::new(1.0, theme::BORDER))
            .corner_radius(12)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                // Right to left so Send keeps its size and the field takes the rest.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_send = self.streaming.is_none() && !self.input.trim().is_empty();
                    let send = ui.add_enabled(
                        can_send,
                        theme::primary_button("Send").min_size(egui::vec2(64.0, 30.0)),
                    );
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .hint_text("Ask something...")
                            .frame(egui::Frame::NONE)
                            .desired_width(ui.available_width())
                            .align(egui::Align2::LEFT_CENTER),
                    );
                    let entered =
                        input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if entered || send.clicked() {
                        self.send();
                        input.request_focus();
                    }
                });
            });
    }

    fn message_list(&self, ui: &mut egui::Ui) {
        let streaming = self
            .streaming
            .as_ref()
            .filter(|s| Some(s.chat_id) == self.current);
        if self.messages.is_empty() && streaming.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                theme::lamp(ui, 40.0);
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Ask anything")
                        .size(19.0)
                        .color(theme::TEXT_STRONG),
                );
                ui.label(
                    egui::RichText::new("The model runs on this laptop. Nothing leaves it.")
                        .color(theme::TEXT_MUTED),
                );
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                reading_column(ui, |ui| {
                    ui.add_space(8.0);
                    for message in &self.messages {
                        message_ui(ui, message.role, &message.content);
                    }
                    if let Some(streaming) = streaming {
                        message_ui(ui, Role::Assistant, &streaming.text);
                    }
                    ui.add_space(8.0);
                });
            });
    }
}

impl MiniApp for AiChat {
    fn name(&self) -> &str {
        "AI Chat"
    }

    fn description(&self) -> &str {
        "Talk to a local AI model, fully offline."
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // Load the model the first time the app is opened.
        if self.worker.is_none() {
            self.worker = Some(Worker::spawn(ui.ctx().clone()));
        }
        self.poll_events();

        egui::Panel::left("chat_list")
            .resizable(true)
            .default_size(220.0)
            .size_range(160.0..=400.0)
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(10, 4)),
            )
            .show(ui, |ui| self.chat_list(ui));

        egui::Panel::bottom("chat_input")
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(20, 14)),
            )
            .show(ui, |ui| self.input_area(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::symmetric(20, 0)),
            )
            .show(ui, |ui| self.message_list(ui));
    }
}

/// One row in the chat list, mapping the shared row widget's actions to this chat.
fn chat_row(
    ui: &mut egui::Ui,
    chat: &ChatSummary,
    selected: bool,
    renaming: &mut Option<(i64, String)>,
) -> Option<ChatAction> {
    let rename = match renaming {
        Some((id, title)) if *id == chat.id => Some(title),
        _ => None,
    };
    list_row(ui, &chat.title, selected, rename).map(|action| match action {
        RowAction::Open => ChatAction::Open(chat.id),
        RowAction::StartRename => ChatAction::StartRename(chat.id),
        RowAction::FinishRename => ChatAction::FinishRename,
        RowAction::CancelRename => ChatAction::CancelRename,
        RowAction::Delete => ChatAction::Delete(chat.id),
    })
}

/// Messages and the input share one centered column, so long lines stay
/// readable on wide windows.
const READING_WIDTH: f32 = 760.0;

fn reading_column(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().min(READING_WIDTH);
    let side = ((ui.available_width() - width) / 2.0).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(side);
        ui.vertical(|ui| {
            ui.set_width(width);
            add_contents(ui);
        });
    });
}

/// One message as a bubble: the user's on the right in amber, the AI's on the
/// left next to the lamp.
fn message_ui(ui: &mut egui::Ui, role: Role, text: &str) {
    const PADDING: egui::Vec2 = egui::vec2(14.0, 10.0);
    const LAMP: f32 = 22.0;
    let is_user = role == Role::User;

    // Lay the text out first so the bubble can hug it, then push the user's
    // bubble to the right by the space left over.
    let (text, color) = match text {
        "" => ("Thinking...", theme::TEXT_FAINT),
        text => (text, theme::TEXT),
    };
    let max_text_width = ui.available_width() * 0.78 - 2.0 * PADDING.x;
    let galley = ui.painter().layout(
        text.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        color,
        max_text_width,
    );
    let bubble_width = galley.size().x + 2.0 * PADDING.x;

    ui.horizontal_top(|ui| {
        if is_user {
            ui.add_space(ui.available_width() - bubble_width);
        } else {
            theme::lamp(ui, LAMP);
        }
        egui::Frame::new()
            .fill(if is_user {
                theme::ACCENT_SOFT
            } else {
                theme::RAISED
            })
            .corner_radius(12)
            .inner_margin(egui::Margin::symmetric(PADDING.x as i8, PADDING.y as i8))
            .show(ui, |ui| ui.add(egui::Label::new(galley)));
    });
    ui.add_space(10.0);
}

/// First line of `text`, shortened to fit in the chat list.
fn truncate_title(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    match line.char_indices().nth(TITLE_MAX_CHARS) {
        Some((end, _)) => format!("{}...", line[..end].trim_end()),
        None => line.to_owned(),
    }
}

#[cfg(test)]
impl AiChat {
    /// A chat app over sample conversations in an in-memory database, with
    /// the first chat open. The model is never loaded.
    pub(super) fn demo() -> Self {
        let store = Store::in_memory();
        let chats: [(&str, &[(Role, &str)]); 4] = [
            ("Morse code basics", &[(Role::User, "How do I signal SOS?")]),
            (
                "Wild edible plants",
                &[(Role::User, "Which plants are safe?")],
            ),
            (
                "Fixing a bike chain",
                &[(Role::User, "My chain keeps slipping.")],
            ),
            (
                "Purifying river water",
                &[
                    (
                        Role::User,
                        "How do I make river water safe to drink without a filter?",
                    ),
                    (
                        Role::Assistant,
                        "Boil it. Let the sediment settle, pour the clear water through a clean \
                         cloth, then keep it at a rolling boil for one minute (three minutes \
                         above 2,000 m). Let it cool covered.",
                    ),
                    (Role::User, "And if I have no fuel?"),
                    (
                        Role::Assistant,
                        "Use sunlight: fill clear plastic bottles, lay them on a reflective \
                         surface in full sun for at least six hours, or two days if it's cloudy. \
                         It works best on water that's already clear.",
                    ),
                ],
            ),
        ];
        for (title, messages) in chats {
            let id = store.create_chat(title).unwrap();
            for &(role, content) in messages {
                let message = Message {
                    role,
                    content: content.to_owned(),
                };
                store.add_message(id, &message).unwrap();
            }
        }

        let mut app = Self {
            store,
            worker: Some(Worker::idle()),
            model: ModelStatus::Ready,
            chats: Vec::new(),
            current: None,
            messages: Vec::new(),
            streaming: None,
            renaming: None,
            notice: None,
            input: String::new(),
        };
        app.refresh_chats();
        let first = app.chats.first().map(|c| c.id);
        if let Some(id) = first {
            app.apply(ChatAction::Open(id));
        }
        app
    }
}
