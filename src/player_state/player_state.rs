#[derive(Debug, Clone)]
pub struct PlayerState {
    pub found_words: Vec<String>,
    pub score: u32,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            found_words: Vec::new(),
            score: 0,
        }
    }

    pub fn add_word(&mut self, word: String) {
        if !self.found_words.contains(&word) {
            self.found_words.push(word);
        }
    }
}
