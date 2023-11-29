use maud::html;
// use std::collections::HashSet;
use std::{sync::Arc, time::Duration};
use tokio::sync::{broadcast, Mutex, Notify};

use crate::boggle::BoggleBoard;
use crate::dictionary::Dictionary;

// Define possible game states
#[derive(Debug, Copy, Clone)]
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
            // state: GameStateEnum::Starting,
            board: None,
            dictionary,
            game_channel_tx,
            state: GameStateEnum::Starting,
            timer: 0,
            timer_cancel_token,
            tx,
        }));

        let game_state_clone = Arc::clone(&game_state);
        tokio::spawn(async move {
            GameState::start_game_loop(game_state_clone).await;
        });

        game_state
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
                            // Break the loop if you want to stop the game loop on game over
                            // break;
                        },
                        // Handle other states if needed
                        _ => {}
                    }
                },
                // Other events can be handled here...
            }
        }
    }
    fn game_over(&self) {
        self.render_new_game_button();
        self.render_valid_words();
        // Implement the logic for when the game is over
    }

    //new_game function will create a new game, it will reset the timer to 0 and intialize a new board
    pub async fn new_game(&mut self) {
        self.state = GameStateEnum::InProgress;
        self.cancel_timer(); // Cancel the existing timer
        self.render_timer("3:00".to_string()).await;
        self.clear_valid_words();
        self.timer = 10;
        self.timer_cancel_token = Arc::new(Notify::new());
        self.board = Some(BoggleBoard::new(Arc::clone(&self.dictionary)));
        self.start_timer();
        self.render_game_board();
    }

    fn start_timer(&self) {
        let timer_tx = self.tx.clone();
        let cancel_token = Arc::clone(&self.timer_cancel_token);
        let game_channel_tx = self.game_channel_tx.clone();

        // Start at 180 seconds (3 minutes)
        let timer = Arc::new(Mutex::new(10));

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
                        let timer_html = html! {
                            div id="game_timer" {
                                (fmt_timer)
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

    pub async fn get_game_state(&self) -> String {
        match self.state {
            GameStateEnum::Starting => {
                println!("Starting");
                html! {
                    div id = "game_timer" {
                        form hx-post="/new_game" {
                            button type="submit" { "New Game" }
                        }
                    }
                    div id="game-board" {}
                    div id="valid-words" {}
                }
                .into_string()
            }
            GameStateEnum::InProgress => {
                println!("In Progress");
                html! {
                    div id = "game_timer" {
                        (self.timer)
                    }
                    div id="game-board" {
                         @if let Some(ref board) = self.board {
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
                }
                .into_string()
            }
            GameStateEnum::GameOver => {
                println!("Game Over");
                html! {
                    div id = "game_timer" {
                        form hx-post="/new_game" {
                            button type="submit" { "New Game" }
                        }
                    }
                    div id="game-board" {
                         @if let Some(ref board) = self.board {
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
                    div id="valid-words" {
                        @if let Some(ref board) = self.board {
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
                }
                .into_string()
            }
        }
    }

    async fn render_timer(&self, value: String) {
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

    fn clear_valid_words(&self) {
        let valid_words_html = html! {
            div id="valid-words" {}
        }
        .into_string();
        self.broadcast_state(valid_words_html);
    }

    fn render_new_game_button(&self) {
        let new_game_button = html! {
            div id = "game_timer" {

                form hx-post="/new_game" {
                    button type="submit" { "New Game" }
                }
            }
        }
        .into_string();
        self.broadcast_state(new_game_button);
    }

    fn broadcast_state(&self, html: String) {
        if let Err(e) = self.tx.send(html) {
            eprintln!("Failed to broadcast game state: {}", e);
        }
    }
}
