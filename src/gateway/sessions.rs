use serde::{ Serialize, Deserialize };
use sea_orm::prelude::Uuid;
use std::hash::{Hash, Hasher};
use std::{ sync::Arc, collections::HashMap };
use tokio::sync::{ RwLock, broadcast::Sender };

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

pub type Table = HashMap<Uuid, Arc<RwLock<User>>>;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    #[serde(skip)]
    pub sender: Sender<String>,
    uuid: Uuid,
    login: String,
    display_name: String,
    created_at: i64,
    #[serde(skip)]
    pub room: Option<String>,
}

impl User {
    pub fn from_account(account: entities::accounts::Model, sender: Sender<String>) -> Self {
        Self { 
            sender,
            uuid: account.uuid,
            login: account.login,
            display_name: account.display_name,
            created_at: account.created_at.timestamp(),
            room: None,
        }
    }

    pub fn set_sender(&mut self, sender: Sender<String>) {
        self.sender = sender;
    }

    pub fn uuid(&self) -> &Uuid {
        &self.uuid
    }

    pub fn login(&self) -> &String {
        &self.login
    }

    pub fn display_name(&self) -> &String {
        &self.display_name
    }

    pub fn created_at(&self) -> &i64 {
        &self.created_at
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