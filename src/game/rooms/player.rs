use std::{borrow::Borrow, hash::Hash};
use serde::Serialize;
use sea_orm::prelude::Uuid;
use tokio::sync::broadcast::Sender;
use crate::gateway::events::SharedTableEvents;
use crate::gateway::payloads::Payload;

#[derive(Debug, Clone, Serialize)]
pub struct Player {
    pub id: Uuid,
    #[serde(skip)]
    pub sender: Sender<String>,
    pub is_ready: bool,
    pub points: u64,
}

impl Player {
    pub fn new(id: Uuid, sender: Sender<String>) -> Self {
        Self { id, sender, is_ready: false, points: 0 }
    }
}

impl SharedTableEvents for Player {
    fn insert(&self, other: Self) {
        let content = Payload::RoomPlayerNew(other).to_json_string();
        let _ = self.sender.send(content);
    }

    fn update(&self, other: Self) {
        let content = Payload::RoomPlayerUpdate(other).to_json_string();
        let _ = self.sender.send(content);
    }

    fn delete(&self, other: Self) {
        let content = Payload::RoomPlayerLeft(other.id).to_json_string();
        let _ = self.sender.send(content);
    }
}

impl Eq for Player {}

impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Player {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Borrow<Uuid> for Player {
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}