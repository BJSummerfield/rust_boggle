use crate::boggle::BoggleBoard;
use axum::extract::ws::Message;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub found_words: Vec<String>,
    pub score: u32,
    pub sender: UnboundedSender<Message>,
    pub valid_words: Vec<(String, String)>,
}

impl PlayerState {
    pub fn new(sender: UnboundedSender<Message>) -> Self {
        Self {
            found_words: Vec::new(),
            score: 0,
            sender,
            valid_words: Vec::new(),
        }
    }

    pub fn add_word(&mut self, word: String) {
        if !self.found_words.contains(&word) {
            self.found_words.push(word);
        }
    }

    pub fn score_words(&mut self, boggle_words: &[(String, String)]) {
        for word in &self.found_words {
            if let Some((found_word, definition)) = boggle_words.iter().find(|(w, _)| w == word) {
                self.score += BoggleBoard::calculate_score(word.len());
                self.valid_words
                    .push((found_word.clone(), definition.clone()));
            }
        }
    }
}
