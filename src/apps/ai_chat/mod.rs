//! AI Chat: conversations with a local LLM, saved in SQLite. A side panel
//! lists past chats, which are auto-named and can be renamed or deleted.

mod store;
mod worker;

use eframe::egui;

use super::MiniApp;
use super::widgets::{RowAction, list_row};
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
        ui.add_space(8.0);
        let new_chat =
            egui::Button::new("+ New chat").min_size(egui::vec2(ui.available_width(), 28.0));
        if ui.add(new_chat).clicked() {
            self.new_chat();
        }
        ui.add_space(4.0);
        ui.separator();

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
            });
        if let Some(action) = action {
            self.apply(action);
        }
    }

    fn input_area(&mut self, ui: &mut egui::Ui) {
        if let Some(notice) = &self.notice {
            ui.colored_label(ui.visuals().warn_fg_color, notice);
            ui.add_space(4.0);
        }
        match &self.model {
            ModelStatus::Loading => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading model...");
                });
            }
            ModelStatus::Failed(err) => {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }
            ModelStatus::Ready => self.input_bar(ui),
        }
    }

    fn input_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let can_send = self.streaming.is_none();
            let send_width = 70.0;
            let input = ui.add_sized(
                [
                    ui.available_width() - send_width - ui.spacing().item_spacing.x,
                    32.0,
                ],
                egui::TextEdit::singleline(&mut self.input)
                    .hint_text("Ask something...")
                    .align(egui::Align2::LEFT_CENTER),
            );
            let entered = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let clicked = ui
                .add_enabled(
                    can_send,
                    egui::Button::new("Send").min_size([send_width, 32.0].into()),
                )
                .clicked();
            if entered || clicked {
                self.send();
                input.request_focus();
            }
        });
    }

    fn message_list(&self, ui: &mut egui::Ui) {
        let streaming = self
            .streaming
            .as_ref()
            .filter(|s| Some(s.chat_id) == self.current);
        if self.messages.is_empty() && streaming.is_none() {
            ui.centered_and_justified(|ui| {
                ui.weak("Ask anything. Everything runs on this laptop.");
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for message in &self.messages {
                    message_ui(ui, message.role, &message.content);
                }
                if let Some(streaming) = streaming {
                    message_ui(ui, Role::Assistant, &streaming.text);
                }
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
            .show(ui, |ui| self.chat_list(ui));

        egui::Panel::bottom("chat_input").show(ui, |ui| {
            ui.add_space(8.0);
            self.input_area(ui);
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ui, |ui| self.message_list(ui));
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

fn message_ui(ui: &mut egui::Ui, role: Role, text: &str) {
    let (who, color) = match role {
        Role::User => ("You", ui.visuals().strong_text_color()),
        Role::Assistant | Role::System => ("AI", ui.visuals().hyperlink_color),
    };
    ui.label(egui::RichText::new(who).strong().color(color));
    if text.is_empty() {
        ui.weak("Thinking...");
    } else {
        ui.label(text);
    }
    ui.add_space(12.0);
}

/// First line of `text`, shortened to fit in the chat list.
fn truncate_title(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    match line.char_indices().nth(TITLE_MAX_CHARS) {
        Some((end, _)) => format!("{}...", line[..end].trim_end()),
        None => line.to_owned(),
    }
}
