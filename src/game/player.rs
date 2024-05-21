use std::{borrow::Borrow, hash::Hash};
use serde::Serialize;
use sea_orm::prelude::Uuid;
use tokio::sync::broadcast::Sender;

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