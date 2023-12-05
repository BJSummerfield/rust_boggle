use axum::extract::ws::Message;
use tokio::sync::mpsc::UnboundedSender;
#[derive(Debug, Clone)]
pub struct PlayerState {
    pub found_words: Vec<String>,
    pub score: u32,
    pub sender: UnboundedSender<Message>,
}

impl PlayerState {
    pub fn new(sender: UnboundedSender<Message>) -> Self {
        Self {
            found_words: Vec::new(),
            score: 0,
            sender,
        }
    }

    pub fn add_word(&mut self, word: String) {
        if !self.found_words.contains(&word) {
            self.found_words.push(word);
        }
    }
}
