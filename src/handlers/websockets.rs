use axum::extract::ws::{Message, WebSocket};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    task::{JoinError, JoinHandle},
};

use crate::models::{Boggle, PlayerId};

#[derive(Deserialize, Debug)]
struct WordSubmission {
    word: String,
}

pub struct WebSockets {}

impl WebSockets {
    pub async fn new(ws: WebSocket, boggle: Arc<Mutex<Boggle>>) {
        println!("websocket connection made");

        //Broadcast tx/rx
        let (sender, mut receiver) = ws.split();

        //Direct tx/rx
        let (ws_sender, ws_receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

        Self::spawn_sender_task(ws_receiver, sender).await;

        Self::handle_new_user(ws_sender.clone(), boggle.clone()).await;

        let username_opt = Self::handle_user_connection(&mut receiver, &ws_sender, &boggle).await;

        match username_opt {
            Some(username) => {
                Self::send_initial_game_boggle(&ws_sender, &boggle).await;
                Self::monitor_websocket_connection(receiver, ws_sender, boggle, username).await;
            }
            None => {
                println!("WebSocket connection closed before username submission.");
            }
        }
    }

    async fn monitor_websocket_connection(
        receiver: SplitStream<WebSocket>,
        ws_sender: UnboundedSender<Message>,
        boggle: Arc<Mutex<Boggle>>,
        username: PlayerId,
    ) {
        let username_clone = username.clone();
        //Sends game messages (html) to all users
        let mut send_task =
            tokio::spawn(Self::spawn_receiver_task(boggle.clone(), ws_sender.clone()));

        //Receives messages from user and sends the user a response
        let mut recv_task = tokio::spawn(Self::receive_messages(
            receiver,
            ws_sender.clone(),
            boggle.clone(),
            username_clone,
        ));

        // Closure to handle task completion
        let handle_task_completion =
            |task_name: &str, other_task: &mut JoinHandle<()>, result: Result<(), JoinError>| {
                match result {
                    Ok(_) => println!("{task_name} task completed"),
                    Err(e) => println!("{task_name} task encountered an error: {:?}", e),
                }
                other_task.abort();
            };

        tokio::select! {
            result = (&mut send_task) => handle_task_completion("Send", &mut recv_task, result),
            result = (&mut recv_task) => handle_task_completion("Receive", &mut send_task, result),
        };

        Self::cleanup(&boggle, &username).await;
    }

    async fn spawn_sender_task(
        mut ws_receiver: UnboundedReceiver<Message>,
        mut sender: SplitSink<WebSocket, Message>,
    ) {
        tokio::spawn(async move {
            while let Some(message) = ws_receiver.recv().await {
                if let Err(error) = sender.send(message).await {
                    println!("Error sending message to WebSocket: {:?}", error);
                    break;
                }
            }
        });
    }

    async fn handle_new_user(ws_sender: UnboundedSender<Message>, boggle: Arc<Mutex<Boggle>>) {
        let new_user_html = boggle.lock().await.get_new_user().await;
        if ws_sender.send(Message::Text(new_user_html)).is_err() {
            println!("Failed to send new user HTML");
        }
    }

    async fn handle_user_connection(
        receiver: &mut SplitStream<WebSocket>,
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
    ) -> Option<PlayerId> {
        while let Some(message_result) = receiver.next().await {
            match message_result {
                Ok(message) => match Self::process_message(message, ws_sender, boggle).await {
                    Ok(Some(username)) => return Some(username),
                    Ok(None) => continue,
                    Err(_) => return None,
                },
                Err(e) => {
                    println!("Error receiving message: {:?}", e);
                    return None;
                }
            }
        }
        None // WebSocket connection closed before username was submitted
    }

    async fn process_message(
        message: Message,
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
    ) -> Result<Option<PlayerId>, String> {
        match message {
            Message::Text(name) => Self::process_text_message(name, ws_sender, boggle).await,
            Message::Close(_) => {
                println!("WebSocket connection closed by client");
                Ok(None)
            }
            _ => {
                println!("Unexpected message type");
                Ok(None)
            }
        }
    }

    async fn process_text_message(
        message: String,
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
    ) -> Result<Option<PlayerId>, String> {
        let player_id = serde_json::from_str::<PlayerId>(&message)
            .map_err(|error| format!("Failed to parse player ID from message: {error}"))?;

        let mut boggle = boggle.lock().await;
        if boggle.players.contains_key(&player_id) {
            let _ = ws_sender.send(Message::Text(format!("{} is taken", player_id)));
            Ok(None)
        } else {
            boggle
                .players
                .add_player(player_id.clone(), ws_sender.clone());
            Ok(Some(player_id))
        }
    }

    async fn send_initial_game_boggle(
        ws_sender: &UnboundedSender<Message>,
        boggle: &Arc<Mutex<Boggle>>,
    ) {
        let initial_game_boggle = boggle.lock().await.get_game_state().await;
        if ws_sender.send(Message::Text(initial_game_boggle)).is_err() {
            println!("Failed to send initial game boggle");
        }
    }

    async fn receive_messages(
        mut receiver: SplitStream<WebSocket>, // Changed to take ownership
        ws_sender: UnboundedSender<Message>,
        boggle: Arc<Mutex<Boggle>>,
        username: PlayerId,
    ) {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            match serde_json::from_str::<WordSubmission>(&text) {
                Ok(word_submission) => {
                    let mut boggle = boggle.lock().await;
                    boggle.submit_word(&username, &word_submission.word);
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
        let mut boggle = boggle.lock().await;
        println!("Cleaning up player: {}", username);
        boggle.players.remove(&username);
        if boggle.players.is_empty() {
            println!("No more players, resetting game boggle");
            boggle.set_state_to_starting().await;
        }
    }
}
