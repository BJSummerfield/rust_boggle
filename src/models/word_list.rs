use super::board::Board;

#[derive(Debug, Clone)]
pub struct WordList {
    words: Vec<(String, String)>,
    pub total_score: u32,
}

impl WordList {
    pub fn new() -> Self {
        WordList {
            words: Vec::new(),
            total_score: 0,
        }
    }

    pub fn add(&mut self, word: &String, definition: String) {
        self.words.push((word.to_string(), definition));
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.iter().any(|(w, _)| w == word)
    }

    pub fn add_from_board_if_not_exists(&mut self, word: &str, board_words: &WordList) {
        if !self.contains(word) {
            if let Some((_, definition)) = board_words.words.iter().find(|(w, _)| w == word) {
                self.add(&word.to_string(), definition.clone());
            }
        }
    }

    pub fn clear(&mut self) {
        self.words.clear();
        self.total_score = 0;
    }

    pub fn total_words(&mut self) {
        for (word, _) in &self.words {
            self.total_score += Board::calculate_score(word.len());
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.words.iter()
    }
}
