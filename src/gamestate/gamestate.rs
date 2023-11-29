// use std::collections::HashSet;
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex, Notify};

use crate::boggle::BoggleBoard;
use crate::dictionary::Dictionary;

use super::boggle_render::*;
//
// Define possible game states
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GameStateEnum {
    Starting,
    InProgress,
    GameOver,
}

//create a game state struct to hold the game state and the broadcast channel sender for sending messages to the clients (players)
#[derive(Debug)]
pub struct GameState {
    // user_set: HashSet<String>,
    state: GameStateEnum,
    board: Option<BoggleBoard>,
    dictionary: Arc<Dictionary>,
    timer: u32,
    timer_cancel_token: Arc<Notify>,
    pub tx: broadcast::Sender<String>,
    game_channel_tx: broadcast::Sender<GameStateEnum>,
}

impl GameState {
    pub fn new() -> Arc<Mutex<Self>> {
        let file_path = format!(
            "{}/static/scrabble-dictionary.txt",
            env!("CARGO_MANIFEST_DIR")
        );
        let dictionary =
            Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));

        let (tx, _) = broadcast::channel(10);
        let (game_channel_tx, _) = broadcast::channel(1);
        let timer_cancel_token = Arc::new(Notify::new());
        let game_state = Arc::new(Mutex::new(Self {
            // user_set: HashSet::new(),
            board: None,
            dictionary,
            game_channel_tx,
            state: GameStateEnum::Starting,
            timer: 10,
            timer_cancel_token,
            tx,
        }));

        let game_state_clone = Arc::clone(&game_state);
        tokio::spawn(async move {
            GameState::start_game_loop(game_state_clone).await;
        });

        game_state
    }

    pub async fn get_game_state(&self) -> String {
        match self.state {
            GameStateEnum::Starting => {
                println!("Starting");
                boggle_render::render_starting_state()
            }
            GameStateEnum::InProgress => {
                println!("In Progress");
                boggle_render::render_inprogress_state(&self.timer.to_string(), &self.board)
            }
            GameStateEnum::GameOver => {
                println!("Game Over");
                boggle_render::render_gameover_state(&self.board)
            }
        }
    }

    pub async fn new_game(&mut self) {
        match self.state {
            GameStateEnum::InProgress => (),
            _ => {
                self.start_timer();

                self.state = GameStateEnum::InProgress;
                self.timer_cancel_token = Arc::new(Notify::new());
                self.board = Some(BoggleBoard::new(Arc::clone(&self.dictionary)));

                let inprogress_html =
                    boggle_render::render_inprogress_state(&self.timer.to_string(), &self.board);
                self.broadcast_state(inprogress_html);
            }
        }
    }

    fn game_over(&self) {
        let game_over_html = boggle_render::render_gameover_state(&self.board);
        self.broadcast_state(game_over_html);
    }

    //submit_word function checks if the word is possible in the board and adds it to the players
    //found words if it is

    pub fn submit_word(&mut self, word: &str) {
        let sanitized_word = word
            .trim()
            .to_lowercase() // Convert to lowercase for consistency
            .chars()
            .filter(|c| c.is_alphabetic()) // Ensure only alphabetic characters
            .collect::<String>();

        // Check word length constraints
        if sanitized_word.len() < 2 || sanitized_word.len() > 16 {
            println!("Word length constraints not met.");
            return;
        }

        println!("Possible word: {}", sanitized_word);
        // // Check if the word is possible
        // if self.board.is_word_possible(&sanitized_word) {
        //     println!("Possible word: {}", sanitized_word);
        // } else {
        //     println!("Word not possible.");
        // }
    }

    pub async fn start_game_loop(game_state: Arc<Mutex<Self>>) {
        let mut game_state_rx = {
            let state = game_state.lock().await;
            state.game_channel_tx.subscribe()
        };

        loop {
            tokio::select! {
                Ok(new_state) = game_state_rx.recv() => {
                    let mut state = game_state.lock().await;
                    state.state = new_state;
                    match new_state {
                        GameStateEnum::GameOver => {
                            state.game_over();
                        },
                        _ => {}
                    }
                },
            }
        }
    }

    fn start_timer(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.timer_cancel_token);
        let game_channel_tx = self.game_channel_tx.clone();

        // Start at 180 seconds (3 minutes)
        let timer = Arc::new(Mutex::new(self.timer));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        let mut timer_guard = timer.lock().await;


                        if *timer_guard == 0 {
                            if let Err(e) = game_channel_tx.send(GameStateEnum::GameOver) {
                                eprintln!("Failed to send game over message: {}", e);
                            }
                            break;
                        }

                         *timer_guard -= 1; // Decrement the timer

                        // Convert the remaining time to minutes and seconds
                        let minutes = *timer_guard / 60;
                        let seconds = *timer_guard % 60;

                        let fmt_timer = format!("{}:{:02}", minutes, seconds);
                        let timer_html = boggle_render::render_timer(&fmt_timer);

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

    fn broadcast_state(&self, html: String) {
        if let Err(e) = self.tx.send(html) {
            eprintln!("Failed to broadcast game state: {}", e);
        }
    }
}
