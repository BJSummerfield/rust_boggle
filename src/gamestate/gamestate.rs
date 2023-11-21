use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Define possible game states
pub enum GameStateEnum {
    Starting,
    InProgress,
    GameOver,
}

// GameState struct
pub struct GameState {
    timer: u32,
    state: GameStateEnum,
}

impl GameState {
    // Constructor for a new GameState
    pub fn new() -> Arc<Mutex<Self>> {
        let game_state = GameState {
            timer: 0,
            state: GameStateEnum::Starting, // initial state
        };

        let arc_state = Arc::new(Mutex::new(game_state));
        let thread_state = Arc::clone(&arc_state);

        // Timer thread
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(1));
            let mut state = thread_state.lock().unwrap();

            match state.state {
                GameStateEnum::Starting => {}
                GameStateEnum::InProgress => {
                    state.timer += 1;
                    println!("Timer: {}", state.timer);
                }
                GameStateEnum::GameOver => {
                    // The timer does not update in this state
                }
            }
        });

        arc_state
    }

    // Method to update the game state
    pub fn update_state(&mut self, new_state: GameStateEnum) {
        self.state = new_state;

        match self.state {
            GameStateEnum::Starting => {
                // Actions to perform when the game is starting
                self.timer = 0; // Example: Reset the timer
                println!("Game is starting, initializing...");
            }
            GameStateEnum::InProgress => {
                // Start or continue the timer
                println!("Game in progress...");
            }
            GameStateEnum::GameOver => {
                // Stop the timer and clean up
                println!("Game over.");
            }
        }
    }
}
