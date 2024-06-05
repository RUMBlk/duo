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
    cards: Vec<Card>,
    cards_count: u16,
}

impl Player {
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
        self.cards_count += 1;
    }

    pub fn remove_card(&mut self, index: usize) -> Card {
        self.cards.remove(index)
    }

    pub fn get_card(&self, index: usize) -> Option<&Card> {
        self.cards.get(index)
    }

    pub fn cards(&self) -> &Vec<Card> {
        &self.cards
    }
}

impl From<rooms::player::Player> for Player {
    fn from(value: rooms::player::Player) -> Self {
        let mut cards: Vec<Card> = Vec::new();
        for _i in 0..8 {
            cards.push(rand::random());
        }
        Self {
            id: value.id,
            sender: value.sender,
            cards: Vec::new(),
            cards_count: 0,
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
    cards_had: u16,
}

impl Loser {
    pub fn new(id: Uuid) -> Self {
        Self { id, points: 0, cards_had: 0 }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn points(&self) -> &u64 {
        &self.points
    }

    pub fn cards_had(&self) -> &u16 {
        &self.cards_had
    }
}

impl From<Player> for Loser {
    fn from(value: Player) -> Self {
        Self { id: value.id, points: 0, cards_had: value.cards_count }
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