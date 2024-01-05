// use std::collections::HashSet;
use crate::boggle::BoggleBoard;
use crate::dictionary::Dictionary;
use crate::player_state::PlayerState;
use crate::render::Render;

use axum::extract::ws::Message;

use maud::html;
use std::{collections::HashMap, env, sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{broadcast, Mutex, Notify};

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
    pub players: HashMap<String, PlayerState>,
    state: GameStateEnum,
    board: BoggleBoard,
    dictionary: Arc<Dictionary>,
    timer: u32,
    timer_cancel_token: Arc<Notify>,
    pub tx: broadcast::Sender<String>,
    game_channel_tx: broadcast::Sender<GameStateEnum>,
}

impl GameState {
    pub fn new() -> Arc<Mutex<Self>> {
        let styles_path =
            env::var("STATIC_FILES_PATH").unwrap_or_else(|_| "/app/static".to_string());
        print!("{}", styles_path);
        let file_path = format!("{}/scrabble-dictionary.txt", styles_path);
        let dictionary =
            Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));

        let (tx, _) = broadcast::channel(10);
        let (game_channel_tx, _) = broadcast::channel(1);
        let timer_cancel_token = Arc::new(Notify::new());
        let game_state = Arc::new(Mutex::new(Self {
            players: HashMap::new(),
            board: BoggleBoard::new(&dictionary),
            dictionary,
            game_channel_tx,
            state: GameStateEnum::Starting,
            timer: 180,
            timer_cancel_token,
            tx,
        }));

        let game_state_clone = Arc::clone(&game_state);
        tokio::spawn(async move {
            GameState::start_game_loop(game_state_clone).await;
        });

        game_state
    }

    fn clear_playerstates(&mut self) {
        for (_, player) in self.players.iter_mut() {
            player.found_words.clear();
            player.valid_words.clear();
            player.score = 0;
        }
    }

    pub fn add_player(&mut self, name: String, sender: UnboundedSender<Message>) {
        self.players.entry(name).or_insert(PlayerState::new(sender));
    }

    pub async fn get_new_user(&self) -> String {
        Render::new_user()
    }

    pub async fn get_game_state(&self) -> String {
        match self.state {
            GameStateEnum::Starting => {
                println!("Starting");
                Render::starting_state()
            }
            GameStateEnum::InProgress => {
                println!("In Progress");

                let minutes = *&self.timer / 60;
                let seconds = *&self.timer % 60;

                let fmt_timer = format!("{}:{:02}", minutes, seconds);
                Render::inprogress_state(&fmt_timer, &self.board)
            }
            GameStateEnum::GameOver => {
                println!("Game Over");
                Render::gameover_state(&self.board, &self.players)
            }
        }
    }

    pub async fn new_game(&mut self) {
        match self.state {
            GameStateEnum::InProgress => (),
            _ => {
                self.clear_playerstates();
                self.start_timer();

                self.state = GameStateEnum::InProgress;
                self.board = BoggleBoard::new(&self.dictionary);

                let minutes = *&self.timer / 60;
                let seconds = *&self.timer % 60;

                let fmt_timer = format!("{}:{:02}", minutes, seconds);
                let inprogress_html = Render::inprogress_state(&fmt_timer, &self.board);
                self.broadcast_state(inprogress_html);
            }
        }
    }

    fn game_over(&mut self) {
        self.total_scores();
        let game_over_html = Render::gameover_state(&self.board, &self.players);
        //total the players word lists
        self.broadcast_state(game_over_html);
    }

    fn total_scores(&mut self) {
        let valid_words = &self.board.valid_words;
        for player in self.players.values_mut() {
            player.score_words(valid_words);
        }
    }

    //submit_word function checks if the word is possible in the board and adds it to the players
    //found words if it is

    pub fn submit_word(&mut self, username: &str, word: &str) {
        let sanitized_word = word.trim().to_uppercase();

        // Check if the word contains spaces or non-alphabetic characters
        if sanitized_word.contains(' ') || sanitized_word.chars().any(|c| !c.is_alphabetic()) {
            println!("Word contains spaces or non-alphabetic characters and is therefore invalid.");
            return;
        }

        // Check word length constraints
        if sanitized_word.len() <= 2 || sanitized_word.len() > 16 {
            println!("Word length constraints not met.");
            return;
        }

        if let Some(player_state) = self.players.get_mut(username) {
            player_state.add_word(sanitized_word); // Add word to player's state
            println!("Player state: {:?}", player_state.found_words);

            // Render the HTML for the submitted word
            let submit_word_html = Render::word_submit(&player_state.found_words);

            // Send the HTML to the specific player
            if let Err(e) = player_state
                .sender
                .send(axum::extract::ws::Message::Text(submit_word_html))
            {
                println!("Failed to send submit word HTML to player: {}", e);
            }
        } else {
            println!("Username not found: {}", username);
        }
    }

    pub async fn set_state_to_starting(&mut self) {
        match self.state {
            GameStateEnum::Starting => (),
            GameStateEnum::InProgress => {
                self.cancel_timer();
                self.state = GameStateEnum::Starting;
            }
            GameStateEnum::GameOver => {
                self.state = GameStateEnum::Starting;
            }
        }
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
                            // Additional logic can be added here if needed
                        },
                        _ => {}
                    }
                },
            }
        }
    }

    pub async fn get_player_score(&self, username: &str) -> String {
        match username {
            "Board Total" => {
                // Assuming self.board is accessible and has a field valid_words
                Render::valid_words(&self.board.valid_words)
            }
            _ => match self.players.get(username) {
                Some(player_state) => Render::valid_words(&player_state.valid_words),
                None => {
                    let markup = html! {
                        div {
                            "Username not found: " (username)
                        }
                    };
                    markup.into_string()
                }
            },
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
                        let timer_html = Render::timer(&fmt_timer);

                        if let Err(e) = timer_tx.send(timer_html) {
                            eprintln!("Failed to send timer update: {}", e);
                        }
                    },
                    _ = cancel_token.notified() => {
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

    fn cancel_timer(&self) {
        self.timer_cancel_token.notify_one();
    }
}
