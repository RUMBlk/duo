use std::{borrow::Borrow, hash::Hash};
use serde::Serialize;

#[derive(Debug, Clone, Eq, Serialize)]
pub struct Player<PlayerData>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    pub data: PlayerData,
    pub is_ready: bool,
    pub points: u64,
}

impl<PlayerData> Player<PlayerData> 
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    pub fn new(data: PlayerData) -> Self {
        Self { data, is_ready: false, points: 0 }
    }
}

impl<PlayerData> From<PlayerData> for Player<PlayerData>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn from(value: PlayerData) -> Self {
        Self::new(value)
    }
}

impl<PlayerData> PartialEq for Player<PlayerData> 
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<PlayerData> Hash for Player<PlayerData> 
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<PlayerData> Borrow<PlayerData> for Player<PlayerData>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn borrow(&self) -> &PlayerData {
        &self.data
    }
}