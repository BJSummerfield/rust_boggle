use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Extension, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use maud::html;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

mod boggle;
mod dictionary;
mod gamestate;
mod player_state;
use gamestate::GameState;

#[tokio::main]
async fn main() {
    let game_state = GameState::new();

    let app = Router::new()
        .route("/", get(serve_boggle_board))
        .route("/new_game", post(new_game_handler))
        // .route("/submit_word", post(submit_word_handler))
        .layer(Extension(Arc::clone(&game_state)))
        // Serve static files from the `static` directory
        .nest_service("/static", ServeDir::new("static"))
        .route("/ws", get(websocket_handler))
        .with_state(game_state);

    // Bind to a socket address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    // Run the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<Mutex<GameState>>>,
) -> impl IntoResponse {
    println!("Websocket connection received");
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(ws: WebSocket, state: Arc<Mutex<GameState>>) {
    let (mut sender, mut receiver) = ws.split();
    let (ws_sender, mut ws_receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Spawn a task for sending messages to the WebSocket
    tokio::spawn(async move {
        while let Some(message) = ws_receiver.recv().await {
            if sender.send(message).await.is_err() {
                println!("Error sending message to WebSocket");
                break;
            }
        }
    });

    let new_user_html = state.lock().await.get_new_user().await;
    if ws_sender.send(Message::Text(new_user_html)).is_err() {
        println!("Failed to send new user HTML");
    }

    let mut username = String::new();

    // Main loop for handling incoming WebSocket messages
    while let Some(Ok(message)) = receiver.next().await {
        println!("initial message received");
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
                    } else {
                        if ws_sender
                            .send(Message::Text(format!("{} is taken", connect.username)))
                            .is_err()
                        {
                            println!("Failed to notify that username is taken");
                        }
                        return;
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
                    break;
                }
            }

            if !username.is_empty() {
                break;
            }
        }
    }

    let initial_game_state = state.lock().await.get_game_state().await;
    if ws_sender.send(Message::Text(initial_game_state)).is_err() {
        println!("Failed to send initial game state");
    }

    let tx = state.lock().await.tx.clone();
    let mut rx = tx.subscribe();

    let ws_sender_clone = ws_sender.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if ws_sender_clone.send(Message::Text(msg)).is_err() {
                break;
            }
        }
    });

    let username_clone = username.clone();
    let ws_sender_clone = ws_sender.clone();
    let mut recv_task = {
        let state = state.clone();
        tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                #[derive(Deserialize, Debug)]
                struct WordSubmission {
                    word: String,
                }

                match serde_json::from_str::<WordSubmission>(&text) {
                    Ok(word_submission) => {
                        let mut gamestate = state.lock().await;
                        gamestate.submit_word(&username_clone, &word_submission.word);
                    }
                    Err(error) => {
                        println!("Failed to parse word message: {error}");
                        if ws_sender_clone
                            .send(Message::Text("Failed to parse word message".to_string()))
                            .is_err()
                        {
                            println!("Failed to send error message");
                        }
                    }
                }
            }
        })
    };

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let mut gamestate = state.lock().await;
    gamestate.players.remove(&username);
    if gamestate.players.is_empty() {
        gamestate.set_state_to_starting().await;
    }
}
async fn new_game_handler(
    Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
) -> impl IntoResponse {
    let mut gamestate = gamestate.lock().await;

    gamestate.new_game().await; // Reset the game state
    (StatusCode::NO_CONTENT, ())
}

async fn serve_boggle_board() -> Html<String> {
    let markup = html! {
        (maud::DOCTYPE)
        html {
            head {
                title { "Boggle Game" }
                script
                    src="https://unpkg.com/htmx.org@1.9.9"
                    integrity="sha384-QFjmbokDn2DjBjq+fM+8LUIVrAgqcNW2s0PjAxHETgRn9l4fvX31ZxDxvwQnyMOX"
                    crossorigin="anonymous" {}
                script src="https://unpkg.com/htmx.org/dist/ext/ws.js" {}
               link rel="stylesheet" href="/static/style.css";
            }
            body hx-ext="ws" ws-connect="/ws" {
                h1 { "Boggle Game" }
                // div hx-ext="ws" ws-connect="/ws" {
                    div id="game_timer" {}
                    div id="game-board" {}
                    div id="word-input" {}
                    div id="valid-words" {}
                // }
            }
        }
    };

    Html(markup.into_string())
}
