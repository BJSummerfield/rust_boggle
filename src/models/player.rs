use crate::models::Board;
use axum::extract::ws::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerIdSubmission {
    pub username: PlayerId,
}

#[derive(Debug, Clone)]
pub struct PlayerList {
    players: HashMap<PlayerId, Player>,
}

impl PlayerList {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    pub fn add_player(
        &mut self,
        id: PlayerId,
        sender: UnboundedSender<Message>,
        username: PlayerId,
    ) {
        self.players
            .entry(id)
            .or_insert(Player::new(sender, username));
        println!("Players: {:?}", self.players);
    }

    pub fn clear_state(&mut self) {
        for player in self.players.values_mut() {
            player.found_words.clear();
            player.score = 0;
            player.valid_words.clear();
        }
    }

    pub fn get_players_sorted_by_score(&self) -> Vec<(&PlayerId, &Player)> {
        let mut sorted_players: Vec<_> = self.players.iter().collect();
        sorted_players.sort_by(|a, b| b.1.score.cmp(&a.1.score));
        sorted_players
    }

    pub fn get(&self, player_id: &PlayerId) -> Option<&Player> {
        self.players.get(player_id)
    }

    pub fn get_mut(&mut self, player_id: &PlayerId) -> Option<&mut Player> {
        self.players.get_mut(player_id)
    }

    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<PlayerId, Player> {
        self.players.values_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn remove(&mut self, player_id: &PlayerId) -> Option<Player> {
        self.players.remove(player_id)
    }

    pub fn contains_key(&self, player_id: &PlayerId) -> bool {
        self.players.contains_key(player_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PlayerId(pub String);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the inner String of PlayerId to the formatter
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub found_words: Vec<String>,
    pub score: u32,
    pub sender: UnboundedSender<Message>,
    pub valid_words: Vec<(String, String)>,
    pub username: PlayerId,
}

impl Player {
    pub fn new(sender: UnboundedSender<Message>, username: PlayerId) -> Self {
        Self {
            found_words: Vec::new(),
            score: 0,
            sender,
            valid_words: Vec::new(),
            username,
        }
    }

    pub fn add_word(&mut self, word: &String) {
        if !self.found_words.contains(&word) {
            self.found_words.push(word.to_string())
        }
    }

    pub fn score_words(&mut self, boggle_words: &[(String, String)]) {
        for word in &self.found_words {
            if let Some((found_word, definition)) = boggle_words.iter().find(|(w, _)| w == word) {
                self.score += Board::calculate_score(word.len());
                self.valid_words
                    .push((found_word.clone(), definition.clone()));
            }
        }
    }
}
