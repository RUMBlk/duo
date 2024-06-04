use sea_orm::prelude::Uuid;
use serde::{ser::SerializeStruct, Serialize};
use std::{ borrow::Borrow, hash::Hash };
use tokio::sync::broadcast::Sender;
use super::card::Card;
use crate::game::rooms;

#[derive(Debug, Clone)]
pub struct Player {
    id: Uuid,
    pub sender: Sender<String>,
    pub cards: Vec<Card>,
    place: u8,
}

impl Player {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn place(&self) -> &u8 {
        &self.place
    }

    pub fn set_place(&mut self, place: u8) -> &Self {
        self.place = place;
        self
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
            place: 0
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