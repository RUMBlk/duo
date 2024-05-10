use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::{collections::{HashMap, HashSet}, ops::Deref, sync::{ Arc, RwLock }, vec, };
use crate::database::queries;
use tokio::sync::broadcast::{ self, Sender };
use random_string;

#[derive(Debug, Serialize, Deserialize)]
pub enum ReturnCode {
    OK,
    BadRequest,
    PlayerAlreadyInRoom,
    PlayerNotInRoom,
    InvalidName,
    InvalidPassword,
    NoOwner,
    Full,
}

#[derive(Debug, Serialize, Clone)]
pub struct Player {
    #[serde(skip_serializing)]
    sender: broadcast::Sender<String>,
    points: u64,
}

impl Player {
    pub fn new(sender: Sender<String>) -> Self {
        Self { sender, points: 0 }
    }
}

pub type Players = HashMap<Uuid, Player>;

fn name_default() -> String { "Room".to_string() }
fn max_players_default() -> usize { 2 }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    #[serde(default = "name_default")]
    name: String,
    pub is_public: bool,
    password: Option<String>,
    owner: Uuid,
    #[serde(default = "max_players_default")]
    max_players: usize,
    #[serde(skip)]
    players: Players,
}

impl Room {
    pub fn new(name: Option<String>, is_public: bool, password: Option<String>, owner: Uuid, max_players: usize) -> Self {
        Self { name: name.unwrap_or(name_default()), is_public: is_public, password, owner, max_players, players: HashMap::new() }
    }

    pub fn generate_id(&self) -> String {
        random_string::generate(6, "0123456789")
    }

    pub fn announce(&self, text: String) {
        for player in self.players.iter() {
            let _ = player.1.sender.send(text.clone());
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn owner(&self) -> Uuid {
        self.owner
    }

    pub fn max_players(&self) -> usize {
        self.max_players
    }

    pub fn players(&self) -> HashMap<Uuid, Player> {
        self.players.clone()
    }

    pub fn set_name(&mut self, name: String) -> Result<ReturnCode, ReturnCode> {
        if !name.is_empty() { self.name = name; Ok(ReturnCode::OK) }
        else { Err(ReturnCode::InvalidName) }
    }

    pub fn set_password(&mut self, password: Option<String>) -> Result<ReturnCode, ReturnCode> {
        if let Some(ref pass) = password {
            if pass.len() < 32 { self.password = password; }
            else { return Err(ReturnCode::InvalidPassword) }
        } 
        Ok(ReturnCode::OK)
    }

    pub fn set_owner(&mut self, player_id: Uuid) -> Result<ReturnCode, ReturnCode> {
        if self.players.contains_key(&player_id) { self.owner = player_id; Ok(ReturnCode::OK) }
        else { Err(ReturnCode::PlayerNotInRoom) }
    }

    pub fn set_max_players(&mut self, max_players: usize) -> Result<ReturnCode, ReturnCode> {
        if max_players < 2 { return Err( ReturnCode::BadRequest ) }
        else { self.max_players = max_players }
        Ok(ReturnCode::OK)
    }

    pub fn join(&mut self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<ReturnCode, ReturnCode> {
        if let Some(pass) = &self.password {
            if Some(pass) != password.as_ref() {
                return Err(ReturnCode::InvalidPassword);
            }
        }
        if self.players.len() >= self.max_players {
            return Err(ReturnCode::Full)
        }

        self.players.insert(player_id, Player::new(sender)).ok_or(ReturnCode::PlayerAlreadyInRoom)?;
        Ok(ReturnCode::OK)
    }

    pub fn leave(&mut self, player_id: Uuid) -> Result<ReturnCode, ReturnCode> {
        let removed = self.players.remove(&player_id).ok_or(ReturnCode::PlayerNotInRoom)?;
        if self.owner == player_id { 
            let next_owner = self.players.iter().next()
                .ok_or(ReturnCode::NoOwner)?;
            self.owner = *next_owner.0;
        }
        Ok(ReturnCode::OK)
    }

}