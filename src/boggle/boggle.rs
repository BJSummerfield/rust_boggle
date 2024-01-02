use crate::dictionary::{Dictionary, SearchResult};
use rand::seq::{IteratorRandom, SliceRandom};
use std::sync::Arc;

// Define the size of the Boggle board
const SIZE: usize = 4;

// Boggle dice configuration
const DICE: [&str; 16] = [
    "AAEEGN", "ELRTTY", "AOOTTW", "ABBJOO", "EHRTVW", "CIMOTU", "DISTTY", "EIOSST", "DELRVY",
    "ACHOPS", "HIMNQU", "EEINSU", "EEGHNW", "AFFKPS", "HLNNRZ", "DEILRX",
];

#[derive(Debug)]
pub struct BoggleBoard {
    pub board: Vec<Vec<char>>,
    dictionary: Arc<Dictionary>,
    pub valid_words: Vec<(String, String)>,
}

impl BoggleBoard {
    // Generate a new Boggle board
    pub fn new(dictionary: &Arc<Dictionary>) -> Self {
        let mut rng = rand::thread_rng();
        let mut dice = DICE;
        dice.shuffle(&mut rng);

        let board_chars: Vec<char> = dice
            .iter()
            .map(|&die| die.chars().choose(&mut rng).unwrap())
            .collect();

        let board: Vec<Vec<char>> = board_chars
            .chunks(SIZE)
            .map(|chunk| chunk.to_vec())
            .collect();

        let mut boggle_board = BoggleBoard {
            board,
            dictionary: dictionary.clone(),
            valid_words: Vec::new(),
        };

        boggle_board.find_valid_words();
        boggle_board
    }

    pub fn find_valid_words(&mut self) {
        let mut visited = vec![vec![false; SIZE]; SIZE];
        let mut current_word = String::new();

        for i in 0..SIZE {
            for j in 0..SIZE {
                self.dfs(i, j, &mut visited, &mut current_word);
            }
        }
    }

    fn dfs(&mut self, i: usize, j: usize, visited: &mut Vec<Vec<bool>>, current_word: &mut String) {
        if i >= SIZE || j >= SIZE || visited[i][j] {
            return;
        }

        visited[i][j] = true;
        let ch = self.board[i][j];
        current_word.push(ch);

        // Check for 'Q' and handle as both 'Q' and 'QU'
        if ch == 'Q' {
            self.check_for_word(i, j, visited, current_word); // Check for 'Q'
            current_word.push('U');
            self.check_for_word(i, j, visited, current_word); // Check for 'QU'
            current_word.pop(); // Remove 'U' after checking
        } else {
            self.check_for_word(i, j, visited, current_word); // Normal processing
        }

        // Backtrack
        current_word.pop();
        visited[i][j] = false;
    }

    fn check_for_word(
        &mut self,
        i: usize,
        j: usize,
        visited: &mut Vec<Vec<bool>>,
        current_word: &mut String,
    ) {
        match self.dictionary.search(&current_word.to_lowercase()) {
            SearchResult::ValidWord(definition) => {
                let word = current_word.clone();
                if !self.valid_words.iter().any(|(w, _)| w == &word) {
                    self.valid_words.push((word, definition));
                }
                // Continue search even after finding a valid word
                self.continue_search(i, j, visited, current_word);
            }
            SearchResult::ValidPrefix => {
                // Continue search for a valid prefix
                self.continue_search(i, j, visited, current_word);
            }
            SearchResult::NotFound => {
                // Stop search if not found
                return;
            }
        }
    }

    fn continue_search(
        &mut self,
        i: usize,
        j: usize,
        visited: &mut Vec<Vec<bool>>,
        current_word: &mut String,
    ) {
        // Define the neighbor offsets
        let row_offsets = [-1, -1, -1, 0, 0, 1, 1, 1];
        let col_offsets = [-1, 0, 1, -1, 1, -1, 0, 1];

        // Iterate over all possible neighbors
        for k in 0..8 {
            let new_i = i as isize + row_offsets[k];
            let new_j = j as isize + col_offsets[k];
            if new_i >= 0 && new_i < SIZE as isize && new_j >= 0 && new_j < SIZE as isize {
                self.dfs(new_i as usize, new_j as usize, visited, current_word);
            }
        }
    }

    pub fn calculate_score(word_length: usize) -> u32 {
        match word_length {
            3 | 4 => 1,
            5 => 2,
            6 => 3,
            7 => 5,
            _ => 11,
        }
    }
}
