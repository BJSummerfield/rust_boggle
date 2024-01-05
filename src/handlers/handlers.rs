use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Form, State,
    },
    http::StatusCode,
    response::{Html, IntoResponse},
    Extension,
};

use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::GameState;

#[derive(serde::Deserialize)]
pub struct PlayerName {
    pub username: String,
}

#[derive(Deserialize, Debug)]
struct WordSubmission {
    word: String,
}
pub struct Handle {}

impl Handle {
    pub async fn new_game(
        Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
    ) -> impl IntoResponse {
        let mut gamestate = gamestate.lock().await;

        gamestate.new_game().await; // Reset the game state
        (StatusCode::NO_CONTENT, ())
    }

    pub async fn get_player_score(
        Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
        Form(PlayerName { username }): Form<PlayerName>,
    ) -> impl IntoResponse {
        let gamestate = gamestate.lock().await;
        let player_score_html = gamestate.get_player_score(&username).await;

        Html(player_score_html).into_response()
    }

    pub async fn websocket(
        ws: WebSocketUpgrade,
        State(state): State<Arc<Mutex<GameState>>>,
    ) -> impl IntoResponse {
        ws.on_upgrade(|socket| async move {
            println!("Websocket connection received");
            Self::init_websocket(socket, state).await
        })
    }

    async fn init_websocket(ws: WebSocket, state: Arc<Mutex<GameState>>) {
        println!("websocket connection made");
        let (sender, mut receiver) = ws.split();
        let (ws_sender, ws_receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

        Self::spawn_sender_task(ws_receiver, sender).await;
        Self::handle_new_user(ws_sender.clone(), state.clone()).await;

        let username = Self::handle_user_connection(&mut receiver, &ws_sender, &state).await;
        Self::send_initial_game_state(&ws_sender, &state).await;
        Self::spawn_receiver_task(&state, ws_sender.clone()).await;
        Self::receive_messages(&mut receiver, &ws_sender, &state, &username).await;

        let mut gamestate = state.lock().await;
        println!("Removing player: {}", username);
        gamestate.players.remove(&username);
        if gamestate.players.is_empty() {
            println!("No more players, resetting game state");
            gamestate.set_state_to_starting().await;
        }
    }

    async fn spawn_sender_task(
        mut ws_receiver: UnboundedReceiver<Message>,
        mut sender: SplitSink<WebSocket, Message>,
        // ws_sender: UnboundedSender<Message>,
    ) {
        tokio::spawn(async move {
            while let Some(message) = ws_receiver.recv().await {
                if sender.send(message).await.is_err() {
                    println!("Error sending message to websocket");
                    // Self::handle_connection_error("Error sending message to WebSocket", &ws_sender);
                    break;
                }
            }
        });
    }

    async fn handle_new_user(ws_sender: UnboundedSender<Message>, state: Arc<Mutex<GameState>>) {
        let new_user_html = state.lock().await.get_new_user().await;
        if ws_sender.send(Message::Text(new_user_html)).is_err() {
            println!("Failed to send new user HTML");
        }
    }

    async fn handle_user_connection(
        receiver: &mut SplitStream<WebSocket>,
        ws_sender: &UnboundedSender<Message>,
        state: &Arc<Mutex<GameState>>,
    ) -> String {
        let mut username = String::new();

        while let Some(Ok(message)) = receiver.next().await {
            if let Message::Text(name) = message {
                #[derive(Deserialize, Debug)]
                struct Connect {
                    username: String,
                }

                match serde_json::from_str::<Connect>(&name) {
                    Ok(connect) => {
                        let mut gamestate = state.lock().await;
                        if !gamestate.players.contains_key(&connect.username) {
                            gamestate.add_player(connect.username.clone(), ws_sender.clone());
                            username = connect.username;
                            println!("username: {}", username);
                            break;
                        } else {
                            if ws_sender
                                .send(Message::Text(format!("{} is taken", connect.username)))
                                .is_err()
                            {
                                println!("Failed to notify that username is taken");
                            }
                            return String::new(); // Username is taken, exit function
                        }
                    }
                    Err(error) => {
                        println!("Failed to parse connect message: {error}");
                        if ws_sender
                            .send(Message::Text("Failed to parse connect message".to_string()))
                            .is_err()
                        {
                            println!("Failed to send error message");
                        }
                        return String::new(); // Error in parsing, exit function
                    }
                }
            }
        }

        username
    }

    async fn send_initial_game_state(
        ws_sender: &UnboundedSender<Message>,
        state: &Arc<Mutex<GameState>>,
    ) {
        let initial_game_state = state.lock().await.get_game_state().await;
        if ws_sender.send(Message::Text(initial_game_state)).is_err() {
            println!("Failed to send initial game state");
        }
    }

    async fn receive_messages(
        receiver: &mut SplitStream<WebSocket>,
        ws_sender: &UnboundedSender<Message>,
        state: &Arc<Mutex<GameState>>,
        username: &str,
    ) {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            match serde_json::from_str::<WordSubmission>(&text) {
                Ok(word_submission) => {
                    let mut gamestate = state.lock().await;
                    gamestate.submit_word(username, &word_submission.word);
                }
                Err(error) => {
                    println!("Failed to parse word message: {error}");
                    if ws_sender
                        .send(Message::Text("Failed to parse word message".to_string()))
                        .is_err()
                    {
                        println!("Failed to send error message");
                    }
                }
            }
        }
    }

    async fn spawn_receiver_task(
        state: &Arc<Mutex<GameState>>,
        ws_sender_clone: UnboundedSender<Message>,
    ) {
        let tx = state.lock().await.tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if ws_sender_clone.send(Message::Text(msg)).is_err() {
                    break;
                }
            }
        });
    }
}
