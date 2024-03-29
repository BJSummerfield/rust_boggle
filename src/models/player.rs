use crate::models::WordList;
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
    }

    pub fn remove_inactive(&mut self) {
        self.players.retain(|_, player| player.active);
    }

    pub fn all_inactive(&self) -> bool {
        self.players.values().all(|player| !player.active)
    }

    pub fn mark_inactive(&mut self, player_id: &PlayerId) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.mark_inactive();
        }
    }

    pub fn mark_active(&mut self, player_id: &PlayerId) {
        if let Some(player) = self.players.get_mut(&player_id) {
            player.mark_active();
        }
    }

    pub fn clear_state(&mut self) {
        for player in self.players.values_mut() {
            player.score = 0;
            player.words.clear();
        }
    }

    pub fn get_players_sorted_by_score(&self) -> Vec<(&PlayerId, &Player)> {
        let mut sorted_players: Vec<_> = self.players.iter().collect();
        sorted_players.sort_by(|a, b| b.1.words.total_score.cmp(&a.1.words.total_score));
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

    pub fn contains_key(&self, player_id: &PlayerId) -> bool {
        self.players.contains_key(player_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PlayerId(pub String);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub score: u32,
    pub sender: UnboundedSender<Message>,
    pub username: PlayerId,
    pub active: bool,
    pub words: WordList,
}

impl Player {
    pub fn new(sender: UnboundedSender<Message>, username: PlayerId) -> Self {
        Self {
            score: 0,
            sender,
            words: WordList::new(),
            username,
            active: true,
        }
    }

    pub fn mark_inactive(&mut self) {
        self.active = false;
    }

    pub fn mark_active(&mut self) {
        self.active = true;
    }
}
