use crate::models::{Board, Dictionary, PlayerId, PlayerList, Timer};
use crate::render::Render;

use maud::html;
use std::{env, sync::Arc};
use tokio::sync::{broadcast, Mutex};

// Define possible game states
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BoggleStateEnum {
    Starting,
    InProgress,
    GameOver,
}

#[derive(Debug)]
pub struct Boggle {
    pub players: PlayerList,
    state: BoggleStateEnum,
    board: Board,
    dictionary: Arc<Dictionary>,
    timer: Arc<Mutex<Timer>>,
    pub tx: broadcast::Sender<String>,
    boggle_channel_tx: broadcast::Sender<BoggleStateEnum>,
}

impl Boggle {
    pub const GAME_DURATION: u32 = 180;

    pub fn new() -> Arc<Mutex<Self>> {
        let styles_path =
            env::var("STATIC_FILES_PATH").unwrap_or_else(|_| "/app/static".to_string());
        let file_path = format!("{}/scrabble-dictionary.txt", styles_path);
        let dictionary =
            Arc::new(Dictionary::new(&file_path).expect("Failed to create dictionary"));

        let (tx, _) = broadcast::channel(10);
        let (boggle_channel_tx, _) = broadcast::channel(1);
        let timer = Timer::new(tx.clone(), boggle_channel_tx.clone());
        let boggle = Arc::new(Mutex::new(Self {
            players: PlayerList::new(),
            board: Board::new(&dictionary),
            dictionary,
            boggle_channel_tx,
            state: BoggleStateEnum::Starting,
            timer,
            tx,
        }));

        let boggle_clone = Arc::clone(&boggle);

        tokio::spawn(async move {
            Boggle::start_game_loop(boggle_clone).await;
        });

        boggle
    }

    pub async fn get_game_state(&self, player_id: &PlayerId) -> String {
        let player = self.players.get(player_id);
        let found_words = match player {
            Some(p) => &p.words,
            None => return "Player not found".to_string(),
        };

        match self.state {
            BoggleStateEnum::Starting => Render::starting_state(),
            BoggleStateEnum::InProgress => {
                let fmt_timer = Timer::format_time(Boggle::GAME_DURATION);
                Render::inprogress_state(&fmt_timer, &self.board, Some(found_words))
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
                self.timer.lock().await.start().await;

                self.state = BoggleStateEnum::InProgress;
                self.board = Board::new(&self.dictionary);

                let fmt_timer = Timer::format_time(Boggle::GAME_DURATION);

                let inprogress_html = Render::inprogress_state(&fmt_timer, &self.board, None);
                self.broadcast_state(inprogress_html);
            }
        }
    }

    fn game_over(&mut self) {
        self.total_scores();
        let game_over_html = Render::gameover_state(&self.board, &self.players);

        self.broadcast_state(game_over_html);
    }

    fn total_scores(&mut self) {
        for player in self.players.values_mut() {
            player.words.total_words();
        }
    }

    pub fn submit_word(&mut self, player_id: &PlayerId, word: &str) -> String {
        let sanitized_word = word.trim().to_uppercase();

        if !Board::is_valid_word(&sanitized_word) {
            return Render::invalid_word_submission();
        }

        let player = match self.players.get_mut(player_id) {
            Some(player) => player,
            None => return Render::invalid_word_submission(),
        };

        player
            .words
            .add_from_board_if_not_exists(&sanitized_word, &self.board.words);

        Render::word_submit(sanitized_word)
    }

    pub async fn set_state_to_starting(&mut self) {
        match self.state {
            BoggleStateEnum::Starting => (),
            BoggleStateEnum::InProgress => {
                self.timer.lock().await.cancel();
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
                        },
                        _ => {}
                    }
                },
            }
        }
    }

    pub async fn get_player_score(&self, username: PlayerId) -> String {
        if username.0 == "Board Total" {
            Render::valid_words(&self.board.words)
        } else {
            match self.players.get(&username) {
                Some(player) => Render::valid_words(&player.words),
                None => {
                    // TODO move this to render
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

    fn broadcast_state(&self, html: String) {
        if let Err(e) = self.tx.send(html) {
            eprintln!("Failed to broadcast game state: {}", e);
        }
    }
}
