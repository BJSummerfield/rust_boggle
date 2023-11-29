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
use std::{net::SocketAddr, sync::Arc};
use tower_http::services::ServeDir;

use tokio::sync::Mutex;
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
    let (mut tx, _) = ws.split();
    println!("Websocket connection established");

    let initial_game_state = { state.lock().await.get_game_state().await };

    if let Err(e) = tx.send(Message::Text(initial_game_state)).await {
        eprintln!("Error sending initial message: {}", e);
        return;
    }
    // Subscribe to the broadcast channel
    let mut rx = {
        let state_locked = state.lock().await;
        state_locked.tx.subscribe()
    };

    // Loop over messages received on the broadcast channel
    while let Ok(msg) = rx.recv().await {
        if tx.send(Message::Text(msg)).await.is_err() {
            break;
        }
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
        body {
            h1 { "Boggle Game" }
            div hx-ext="ws" ws-connect="/ws" {}
                div id="game_timer" {}
                div id="game-board" {}
                div id="word-input" {
                 input type="text" placeholder="Enter word" hx-post="/submit-word" {}
                }
                div id="valid-words" {}
            }
        }
    };

    Html(markup.into_string())
}
