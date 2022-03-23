pub enum Event {
    ChatMessage(ChatMessage),
}

pub struct ChatMessage {
    pub user_name: String,
    pub msg: String,
}
