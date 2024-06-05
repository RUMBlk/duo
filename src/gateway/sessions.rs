use serde::{ Serialize, Deserialize };
use sea_orm::prelude::Uuid;
use std::{
    hash::{Hash, Hasher},
    borrow::Borrow,
};
use tokio::sync::broadcast::Sender;
use crate::database::entities;

#[derive(Debug, Serialize, Deserialize)]
pub enum ReturnCode {
    OK,
    PlayerAlreadyInRoom,
    PlayerNotInRoom,
    InvalidName,
    InvalidPassword,
    NoOwner,
    Full,
    MaxPlayersNotSet,
    MaxPlayersCantBeLowerThan(usize),
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    #[serde(skip)]
    pub sender: Sender<String>,
    uuid: Uuid,
    pub room: Option<String>,
}

impl User {
    pub fn from_account(account: entities::accounts::Model, sender: Sender<String>) -> Self {
        Self { 
            sender,
            uuid: account.uuid,
            room: None,
        }
    }

    pub fn set_sender(&mut self, sender: Sender<String>) {
        self.sender = sender;
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }
}

impl Eq for User { }

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Hash for User {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl Borrow<Uuid> for User {
    fn borrow(&self) -> &Uuid {
        &self.uuid
    }
}