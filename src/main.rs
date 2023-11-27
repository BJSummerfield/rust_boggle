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
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::services::ServeDir;

mod boggle;
mod dictionary;
use dictionary::Dictionary;
mod gamestate;
use gamestate::GameState;

#[tokio::main]
async fn main() {
    let file_path = format!(
        "{}/static/scrabble-dictionary.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let dictionary = Dictionary::new(&file_path).expect("Failed to create dictionary");

    let game_state = Arc::new(Mutex::new(GameState::new(Arc::new(dictionary))));

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
    // Subscribe to the broadcast channel
    let mut rx = {
        let state_locked = state.lock().unwrap_or_else(|e| e.into_inner());
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
    let mut gamestate = gamestate.lock().unwrap_or_else(|e| e.into_inner());
    gamestate.new_game(); // Reset the game state
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
                form hx-post="/new_game" {
                    button type="submit" { "New Game" }
                }
                div id="game-board" {}
            }
            div id="valid-words" { }
        }
    };

    Html(markup.into_string())
}
