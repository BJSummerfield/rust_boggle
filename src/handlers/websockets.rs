use axum::extract::ws::{Message, WebSocket};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};

use std::sync::Arc;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::{JoinError, JoinHandle},
};
use tower_sessions::Session;

use crate::models::{Boggle, PlayerId};

pub struct WebSockets {}

impl WebSockets {
    pub async fn new(ws: WebSocket, boggle: Arc<Mutex<Boggle>>, session: Session) {
        //Broadcast tx/rx
        let (sender, receiver) = ws.split();
        //Direct tx/rx
        let (ws_sender, ws_receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

        Self::spawn_sender_task(ws_receiver, sender).await;

        let player_id_opt = Self::handle_user_connection(&ws_sender, &boggle, &session).await;

        match player_id_opt {
            Some(player_id) => {
                Self::send_initial_game_boggle(&ws_sender, &boggle, &player_id).await;
                Self::monitor_websocket_connection(receiver, ws_sender, boggle, player_id).await;
            }
            None => {
                //handle reconnection and break websocket connection
            }
        }
    }

    async fn spawn_sender_task(
        mut ws_receiver: UnboundedReceiver<Message>,
        mut sender: SplitSink<WebSocket, Message>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.recv().await {
                if let Err(error) = sender.send(msg).await {
                    eprintln!("Failed to send message: {:?}", error);
                    break;
                }
            }
        })
    }
    async fn monitor_websocket_connection(
        receiver: SplitStream<WebSocket>,
        ws_sender: UnboundedSender<Message>,
        boggle: Arc<Mutex<Boggle>>,
        username: PlayerId,
    ) {
        //Sends game messages (html) to all users
        let mut send_task =
            tokio::spawn(Self::spawn_receiver_task(boggle.clone(), ws_sender.clone()));

        //Receives messages from user and sends the user a response
        let mut recv_task = tokio::spawn(Self::receive_messages(receiver));

        // Closure to handle task completion
        let handle_task_completion =
            |task_name: &str, other_task: &mut JoinHandle<()>, result: Result<(), JoinError>| {
                match result {
                    Ok(_) => {}
                    Err(e) => eprintln!("{task_name} task encountered an error: {:?}", e),
                }
                other_task.abort();
            };

        tokio::select! {
            result = (&mut send_task) => handle_task_completion("Send", &mut recv_task, result),
            result = (&mut recv_task) => handle_task_completion("Receive", &mut send_task, result),
        };

        Self::cleanup(&boggle, &username).await;
    }

    async fn handle_user_connection(
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
        session: &Session,
    ) -> Option<PlayerId> {
        let player_id = session
            .get("id")
            .await
            .unwrap()
            .expect("No session_id found");

        let username: PlayerId = session
            .get("username")
            .await
            .unwrap()
            .expect("No username found");

        let mut boggle = boggle.lock().await;
        if boggle.players.contains_key(&player_id) {
            boggle.players.mark_active(&player_id);
            Some(player_id)
        } else {
            boggle
                .players
                .add_player(player_id.clone(), ws_sender.clone(), username.clone());
            Some(player_id)
        }
    }

    async fn send_initial_game_boggle(
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
        player_id: &PlayerId,
    ) {
        let initial_game_boggle = boggle.lock().await.get_game_state(player_id).await;
        // Attempt to send the message and capture the error if it occurs
        if let Err(e) = ws_sender.send(Message::Text(initial_game_boggle)) {
            eprintln!("Failed to send initial game boggle: {:?}", e);
        }
    }

    async fn receive_messages(mut receiver: SplitStream<WebSocket>) {
        while let Some(message) = receiver.next().await {
            match message {
                Ok(Message::Close(_)) => {
                    eprintln!("WebSocket connection closed by client.");
                    break;
                }
                Ok(_) => {
                    // Ignore other messages.
                }
                Err(e) => {
                    eprintln!("Error in WebSocket connection: {:?}", e);
                    break;
                }
            }
        }
    }

    async fn spawn_receiver_task(
        boggle: Arc<Mutex<Boggle>>,
        ws_sender_clone: UnboundedSender<Message>,
    ) {
        let tx = boggle.lock().await.tx.clone();
        let mut rx = tx.subscribe();

        while let Ok(msg) = rx.recv().await {
            if ws_sender_clone.send(Message::Text(msg)).is_err() {
                break;
            }
        }
    }

    async fn cleanup(boggle: &Arc<Mutex<Boggle>>, username: &PlayerId) {
        println!("Cleaning up player: {:?}", username);
        let mut boggle = boggle.lock().await;

        boggle.players.mark_inactive(&username);

        if boggle.players.all_inactive() {
            boggle.players.remove_inactive();
            boggle.set_state_to_starting().await;
        }
    }
}
