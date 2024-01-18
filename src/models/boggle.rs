use crate::models::{Board, Dictionary, PlayerId, PlayerList};
use crate::render::Render;

use maud::html;
use std::{env, sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex, Notify};

// Define possible game states
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BoggleStateEnum {
    Starting,
    InProgress,
    GameOver,
}

//create a game state struct to hold the game state and the broadcast channel sender for sending messages to the clients (players)
#[derive(Debug)]
pub struct Boggle {
    pub players: PlayerList,
    state: BoggleStateEnum,
    board: Board,
    dictionary: Arc<Dictionary>,
    timer: u32,
    timer_cancel_token: Arc<Notify>,
    pub tx: broadcast::Sender<String>,
    boggle_channel_tx: broadcast::Sender<BoggleStateEnum>,
}

impl Boggle {
    pub fn new() -> Arc<Mutex<Self>> {
        let styles_path =
            env::var("STATIC_FILES_PATH").unwrap_or_else(|_| "/app/static".to_string());
        let file_path = format!("{}/scrabble-dictionary.txt", styles_path);
        let dictionary =
            Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));

        let (tx, _) = broadcast::channel(10);
        let (boggle_channel_tx, _) = broadcast::channel(1);
        let timer_cancel_token = Arc::new(Notify::new());
        let boggle = Arc::new(Mutex::new(Self {
            players: PlayerList::new(),
            board: Board::new(&dictionary),
            dictionary,
            boggle_channel_tx,
            state: BoggleStateEnum::Starting,
            timer: 10,
            timer_cancel_token,
            tx,
        }));

        let boggle_clone = Arc::clone(&boggle);
        tokio::spawn(async move {
            Boggle::start_game_loop(boggle_clone).await;
        });

        boggle
    }

    pub async fn get_new_user(&self) -> String {
        Render::new_user()
    }

    pub async fn get_game_state(&self, player_id: &PlayerId) -> String {
        let player = self.players.get(player_id);
        let found_words = match player {
            Some(p) => &p.found_words,
            None => return "Player not found".to_string(),
        };

        match self.state {
            BoggleStateEnum::Starting => Render::starting_state(),
            BoggleStateEnum::InProgress => {
                let minutes = *&self.timer / 60;
                let seconds = *&self.timer % 60;

                let fmt_timer = format!("{}:{:02}", minutes, seconds);
                Render::inprogress_state(&fmt_timer, &self.board, found_words)
            }
            BoggleStateEnum::GameOver => Render::gameover_state(&self.board, &self.players),
        }
    }

    pub async fn new_game(&mut self) {
        match self.state {
            BoggleStateEnum::InProgress => (),
            _ => {
                self.players.remove_inactive();
                self.players.clear_state();
                self.start_timer();

                self.state = BoggleStateEnum::InProgress;
                self.board = Board::new(&self.dictionary);

                let minutes = *&self.timer / 60;
                let seconds = *&self.timer % 60;

                let fmt_timer = format!("{}:{:02}", minutes, seconds);
                let inprogress_html = Render::inprogress_state(&fmt_timer, &self.board, &vec![]);
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

    pub fn submit_word(&mut self, player_id: &PlayerId, word: &str) -> String {
        let sanitized_word = word.trim().to_uppercase();

        if !Boggle::is_valid_word(&sanitized_word) {
            return Render::invalid_word_submission();
        }

        let player = match self.players.get_mut(player_id) {
            Some(player) => player,
            None => return Render::invalid_word_submission(),
        };

        player.add_word(&sanitized_word);
        Render::word_submit(sanitized_word)
    }

    //maybe move this to the boggleboard
    fn is_valid_word(word: &str) -> bool {
        !(word.contains(' ')
            || word.chars().any(|c| !c.is_alphabetic())
            || word.len() <= 2
            || word.len() > 16)
    }

    pub async fn set_state_to_starting(&mut self) {
        match self.state {
            BoggleStateEnum::Starting => (),
            BoggleStateEnum::InProgress => {
                self.cancel_timer();
                self.state = BoggleStateEnum::Starting;
            }
            BoggleStateEnum::GameOver => {
                self.state = BoggleStateEnum::Starting;
            }
        }
    }

    pub async fn start_game_loop(boggle: Arc<Mutex<Self>>) {
        let mut boggle_rx = {
            let state = boggle.lock().await;
            state.boggle_channel_tx.subscribe()
        };

        loop {
            tokio::select! {
                Ok(new_state) = boggle_rx.recv() => {
                    let mut state = boggle.lock().await;
                    state.state = new_state;
                    match new_state {
                        BoggleStateEnum::GameOver => {
                            state.game_over();
                            // Additional logic can be added here if needed
                        },
                        _ => {}
                    }
                },
            }
        }
    }

    pub async fn get_player_score(&self, username: PlayerId) -> String {
        if username.0 == "Board Total" {
            // Assuming self.board is accessible and has a field valid_words
            Render::valid_words(&self.board.valid_words)
        } else {
            match self.players.get(&username) {
                // Note the reference '&username'
                Some(player) => Render::valid_words(&player.valid_words),
                None => {
                    let markup = html! {
                        div {
                            "Username not found: " (username.0)
                        }
                    };
                    markup.into_string()
                }
            }
        }
    }

    fn start_timer(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.timer_cancel_token);
        let boggle_channel_tx = self.boggle_channel_tx.clone();

        // Start at 180 seconds (3 minutes)
        let timer = Arc::new(Mutex::new(self.timer));
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        let mut timer_guard = timer.lock().await;

                        if *timer_guard == 0 {
                            if let Err(e) = boggle_channel_tx.send(BoggleStateEnum::GameOver) {
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
