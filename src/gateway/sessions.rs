use serde::{ Serialize, Deserialize };
use sea_orm::prelude::Uuid;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(skip)]
    pub sender: Option<Sender<String>>,
    uuid: Uuid,
    login: String,
    display_name: String,
    created_at: i64,
    pub room: Option<String>,
}

impl User {
    pub fn set_sender(&mut self, sender: Sender<String>) {
        self.sender = Some(sender);
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

    pub fn room(&self) -> &Option<String> {
        &self.room
    }
}

impl From<entities::accounts::Model> for User {
    fn from(model: entities::accounts::Model) -> Self {
        let uuid = model.uuid;
        let login = model.login;
        let display_name = model.display_name;
        let created_at = model.created_at.timestamp();
        Self { sender: None, uuid, login, display_name, created_at, room: None }
    }
}