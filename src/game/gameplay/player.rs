use sea_orm::prelude::Uuid;
use serde::{ser::SerializeStruct, Serialize};
use std::{ borrow::Borrow, hash::Hash, ops::Deref };
use tokio::sync::broadcast::Sender;
use super::card::Card;
use crate::game::rooms;

#[derive(Debug, Clone)]
pub struct Player {
    id: Uuid,
    pub sender: Sender<String>,
    pub cards: Vec<Card>,
}

impl Player {
    pub fn id(&self) -> &Uuid {
        &self.id
    }
}

impl From<rooms::player::Player> for Player {
    fn from(value: rooms::player::Player) -> Self {
        let mut cards: Vec<Card> = Vec::new();
        for i in 0..8 {
            cards.push(rand::random());
        }
        Self {
            id: value.id,
            sender: value.sender,
            cards: Vec::new(),
        }
    }
}

impl Hash for Player {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Player { }
impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Borrow<Uuid> for Player {
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}

impl Serialize for Player {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("Player", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("cards", &self.cards.len())?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Loser {
    id: Uuid,
    points: u64,
}

impl Loser {
    pub fn new(id: Uuid) -> Self {
        Self { id, points: 0 }
    }

    pub fn get(&self) -> (Uuid, u64) {
        (self.id, self.points)
    }
}

impl From<Player> for Loser {
    fn from(value: Player) -> Self {
        Self { id: value.id, points: 0 }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Losers(Vec<Loser>);

impl From<Vec<Loser>> for Losers {
    fn from(value: Vec<Loser>) -> Self {
        let len = value.len();
        let mut losers = Vec::new();
        for (i, loser) in value.iter().enumerate() {
            let mut loser = loser.clone();
            loser.points = ((len*10)*i/len) as u64;
            losers.push(loser);
        }
        Self(losers)
    }
}

impl Deref for Losers {
    type Target = Vec<Loser>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}