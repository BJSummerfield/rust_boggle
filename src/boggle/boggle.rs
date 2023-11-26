use crate::dictionary::{Dictionary, SearchResult};
use maud::html;
use rand::seq::{IteratorRandom, SliceRandom};
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::sync::{broadcast, Notify};
use tokio::time::{sleep, Duration};

// Define the size of the Boggle board
const SIZE: usize = 4;

// Boggle dice configuration
const DICE: [&str; 16] = [
    "AAEEGN", "ELRTTY", "AOOTTW", "ABBJOO", "EHRTVW", "CIMOTU", "DISTTY", "EIOSST", "DELRVY",
    "ACHOPS", "HIMNQU", "EEINSU", "EEGHNW", "AFFKPS", "HLNNRZ", "DEILRX",
];

pub struct BoggleBoard {
    pub board: Vec<Vec<char>>,
    dictionary: Arc<Dictionary>,
    pub valid_words: Vec<(String, String)>,
    timer: u32,
    pub tx: broadcast::Sender<String>,
    timer_cancel_token: Arc<Notify>,
}

impl BoggleBoard {
    // Generate a new Boggle board
    pub fn new(dictionary: Arc<Dictionary>) -> Self {
        let (tx, _) = broadcast::channel(10);
        let timer_cancel_token = Arc::new(Notify::new());

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
            dictionary,
            valid_words: Vec::new(),
            timer: 0,
            tx,
            timer_cancel_token,
        };

        boggle_board.start_timer();
        boggle_board.find_valid_words();
        boggle_board
    }

    pub fn start_timer(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.timer_cancel_token);
        let timer = Arc::new(Mutex::new(self.timer));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = sleep(Duration::from_secs(1)) => {
                        let mut timer_guard = match timer.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => {
                                eprintln!("Mutex was poisoned. Using poisoned data.");
                                poisoned.into_inner()
                            }
                        };
                        *timer_guard += 1;
                        let timer_html = html! {
                            div id="game_timer" {
                                (*timer_guard)
                            }
                        }.into_string();

                        if let Err(e) = timer_tx.send(timer_html) {
                            eprintln!("Failed to send timer update: {}", e);
                        }
                    },
                    _ = cancel_token.notified() => {
                        // Exit the loop when notified
                        break;
                    }
                }
            }
        });
    }

    pub fn cancel_timer(&self) {
        self.timer_cancel_token.notify_one();
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
        current_word.push(self.board[i][j]);

        match self.dictionary.search(&current_word.to_lowercase()) {
            SearchResult::ValidWord(definition) => {
                let word = current_word.clone();
                if !self.valid_words.iter().any(|(w, _)| w == &word) {
                    self.valid_words.push((word, definition));
                }
            }
            SearchResult::ValidPrefix => {
                // Explore all 8 adjacent cells
                let row_offsets = [-1, -1, -1, 0, 0, 1, 1, 1];
                let col_offsets = [-1, 0, 1, -1, 1, -1, 0, 1];

                for k in 0..8 {
                    let new_i = i as isize + row_offsets[k];
                    let new_j = j as isize + col_offsets[k];
                    if new_i >= 0 && new_i < SIZE as isize && new_j >= 0 && new_j < SIZE as isize {
                        self.dfs(new_i as usize, new_j as usize, visited, current_word);
                    }
                }
            }
            SearchResult::NotFound => {
                // Early return since the path won't lead to a valid word
                current_word.pop();
                visited[i][j] = false;
                return;
            }
        }

        // Backtrack
        current_word.pop();
        visited[i][j] = false;
    }
}
//
// Implement Display for BoggleBoard to print the board
impl fmt::Display for BoggleBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in &self.board {
            for &ch in row {
                write!(f, "{} ", ch)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
