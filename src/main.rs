use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Extension, Form, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use maud::html;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;

use tokio::sync::{broadcast, Mutex};

use serde::Deserialize;

mod boggle;
mod dictionary;
mod gamestate;
use gamestate::GameState;

#[tokio::main]
async fn main() {
    let game_state = GameState::new();

    let app = Router::new()
        .route("/", get(serve_boggle_board))
        .route("/new_game", post(new_game_handler))
        .route("/submit_word", post(submit_word_handler))
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
// websocket function for handling sending websocket messages to clients
async fn websocket(ws: WebSocket, state: Arc<Mutex<GameState>>) {
    let (mut sender, mut reciever) = ws.split();
    println!("Websocket connection established");

    let new_user_html = { state.lock().await.get_new_user().await };
    let _ = sender.send(Message::Text(new_user_html)).await;
    let mut tx = None::<broadcast::Sender<String>>;
    let mut username = String::new();

    while let Some(Ok(message)) = reciever.next().await {
        println!("message received {:?} ", message);
        if let Message::Text(name) = message {
            println!("Name here {name}");
            #[derive(Deserialize, Debug)]
            struct Connect {
                username: String,
            }

            let connect: Connect = match serde_json::from_str(&name) {
                Ok(connect) => connect,
                Err(error) => {
                    println!("{error}");
                    let _ = sender
                        .send(Message::Text(String::from(
                            "Failed to parse connect message",
                        )))
                        .await;
                    break;
                }
            };
            println!("connect here {:?}", connect);
            // Scope to drop the mutex guard before the next await
            {
                // If username that is sent by client is not taken, fill username string.
                let mut gamestate = state.lock().await;

                tx = Some(gamestate.tx.clone());

                println!("gamestate.players {:?}", gamestate.players);
                if !gamestate.players.contains(&connect.username) {
                    gamestate.players.insert(connect.username.to_owned());
                    username = connect.username;
                }
                println!("After gamstate.players {:?}", gamestate.players);
            }

            // If not empty we want to quit the loop else we want to quit function.
            if tx.is_some() && !username.is_empty() {
                break;
            } else {
                // Only send our client that username is taken.
                let _ = sender
                    .send(Message::Text(format!("{} is taken", username)))
                    .await;

                return;
            }
        }
    }

    let tx = tx.unwrap();
    let mut rx = tx.subscribe();
    let msg = format!("{} joined.", username);
    let _ = tx.send(msg);

    let initial_game_state = { state.lock().await.get_game_state().await };
    let _ = tx.send(initial_game_state).is_err();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = {
        // Clone things we want to pass to the receiving task.
        let tx = tx.clone();

        // This task will receive messages from client and send them to broadcast subscribers.
        tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = reciever.next().await {
                if tx.send(text).is_err() {
                    break;
                }
            }
        })
    };

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    let msg = format!("{} left.", username);
    let _ = tx.send(msg);
    let mut gamestate = state.lock().await;

    gamestate.players.remove(&username);
}

async fn new_game_handler(
    Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
) -> impl IntoResponse {
    let mut gamestate = gamestate.lock().await;

    gamestate.new_game().await; // Reset the game state
    (StatusCode::NO_CONTENT, ())
}

async fn submit_word_handler(
    Extension(gamestate): Extension<Arc<Mutex<GameState>>>,
    Form(word_data): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut gamestate = gamestate.lock().await;
    let submitted_word = match word_data.get("word") {
        Some(word) => word.to_owned(),
        None => String::new(), // Provide a default empty String if word does not exist
    };
    gamestate.submit_word(&submitted_word); // Reset the game state
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
