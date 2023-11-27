use maud::html;
// use std::collections::HashSet;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{broadcast, Notify};

use crate::boggle::BoggleBoard;
use crate::dictionary::Dictionary;

// Define possible game states
// #[derive(Debug)]
// pub enum GameStateEnum {
//     Starting,
//     // InProgress,
//     // GameOver,
// }

//create a game state struct to hold the game state and the broadcast channel sender for sending messages to the clients (players)
#[derive(Debug)]
pub struct GameState {
    // user_set: HashSet<String>,
    // state: GameStateEnum,
    board: Option<BoggleBoard>,
    dictionary: Arc<Dictionary>,
    timer: u32,
    timer_cancel_token: Arc<Notify>,
    pub tx: broadcast::Sender<String>,
}

impl GameState {
    pub fn new(dictionary: Arc<Dictionary>) -> Self {
        let (tx, _) = broadcast::channel(10);
        let timer_cancel_token = Arc::new(Notify::new());
        Self {
            // user_set: HashSet::new(),
            // state: GameStateEnum::Starting,
            board: None,
            dictionary,
            timer: 0,
            timer_cancel_token,
            tx,
        }
    }

    //new_game function will create a new game, it will reset the timer to 0 and intialize a new board
    pub fn new_game(&mut self) {
        self.render_timer(0);
        self.cancel_timer(); // Cancel the existing timer

        self.timer = 0;
        self.timer_cancel_token = Arc::new(Notify::new());

        self.board = Some(BoggleBoard::new(Arc::clone(&self.dictionary)));

        self.start_timer();
        self.render_game_board();
        self.render_valid_words();
    }

    fn start_timer(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.timer_cancel_token);
        let timer = Arc::new(Mutex::new(0)); // Reset timer value

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        let mut timer_guard = timer.lock().unwrap();
                        *timer_guard += 1;
                        println!("Timer: {}", *timer_guard);

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
                        println!("Timer cancelled");
                        break;
                    }
                }
            }
        });
    }

    fn cancel_timer(&self) {
        self.timer_cancel_token.notify_one();
    }

    fn render_timer(&self, value: u32) {
        let timer_html = html! {
            div id="game_timer" {
                (value)
            }
        }
        .into_string();
        self.broadcast_state(timer_html);
    }
    fn render_game_board(&self) {
        if let Some(ref board) = self.board {
            let board_html = html! {
                div id="game-board" {
                    @for row in &board.board {
                        div class="board-row" {
                            @for &letter in row {
                                div class="board-cell" {
                                    (letter)
                                }
                            }
                        }
                    }
                }
            }
            .into_string();
            self.broadcast_state(board_html);
        }
    }

    fn render_valid_words(&self) {
        if let Some(ref board) = self.board {
            let valid_words_html = html! {
                div id="valid-words" {
                    ul {
                        @for (word, definition) in &board.valid_words {
                            li {
                                div class="word-container" {
                                    span class="word" { (word) }
                                    span class="definition" { (definition) }
                                }
                            }
                        }
                    }
                }
            }
            .into_string();
            self.broadcast_state(valid_words_html);
        }
    }

    fn broadcast_state(&self, html: String) {
        if let Err(e) = self.tx.send(html) {
            eprintln!("Failed to broadcast game state: {}", e);
        }
    }
}
