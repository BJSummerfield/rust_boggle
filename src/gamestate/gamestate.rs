use maud::html;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time;
// Define possible game states
pub enum GameStateEnum {
    Starting,
    InProgress,
    GameOver,
}

//create a game state struct to hold the game state and the broadcast channel sender for sending messages to the clients (players)
pub struct GameState {
    user_set: HashSet<String>,
    timer: u32,
    state: GameStateEnum,
    pub tx: broadcast::Sender<String>,
}

// Implement the GameState struct with a new function well want to also create a funciont to
// increment the timer by 1 every second.  The timer is what we'll be broadcasting to the clients

impl GameState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(10);
        Self {
            user_set: HashSet::new(),
            timer: 0,
            state: GameStateEnum::Starting,
            tx,
        }
    }

    //create a function that increments the timer field everyone second.  We'll spawn this function in the main function  and it will run in the background for the duration of the program
    //our timer will be broadcasted to the websockets everytime we increment

    pub fn start_timer(game_state: Arc<Mutex<Self>>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                let mut state = match game_state.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        eprintln!("Mutex was poisoned. Using poisoned data.");
                        poisoned.into_inner()
                    }
                };
                state.timer += 1;
                let timer_html = state.timer();
                if let Err(e) = state.tx.send(timer_html) {
                    eprintln!("Failed to send timer update: {}", e);
                }
            }
        });
    }

    //create a function that generates the maud html template that has a div with the id of "timer"
    //and the contents are the timer value
    pub fn timer(&self) -> String {
        html! {
            div id="game_timer" {
                (self.timer)
            }
        }
        .into_string()
    }
}
