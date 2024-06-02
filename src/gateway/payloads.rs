use sea_orm::prelude::Uuid;
use serde::{ Serialize, Deserialize };
use serde_json;
use crate::game;

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
    //From Server
    #[serde(skip_deserializing)]
    Error(Error),
    #[serde(skip_deserializing)]
    Hello(Hello),
    #[serde(skip_deserializing)]
    RoomPlayerNew(RoomPlayer),
    #[serde(skip_deserializing)]
    RoomPlayerUpdate(RoomPlayer),
    #[serde(skip_deserializing)]
    RoomPlayerLeft(RoomPlayerInfo),
    #[serde(skip_deserializing)]
    RoomCreate(game::rooms::Room),
    #[serde(skip_deserializing)]
    RoomUpdate(game::rooms::Room),
    //From Server/Client
    Identify(Identify),
    #[serde(skip_deserializing)]
    Ready(super::sessions::User),
    /*//From Client
    RoomJoin(RoomJoin),
    RoomLeave(String),*/
}

impl Payload {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize Gateway Payload")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
        BadRequest(String),
        Declined,
        BadToken,
        InvalidToken,
        InternalServerError,
        NotFound,
        Forbidden,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hello {
    heartbeat_interval: u64, 
}

impl Hello {
    pub fn new(heartbeat_interval: u64) -> Self {
        Self { heartbeat_interval }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identify {
    token: String,
}

impl Identify {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}

#[derive(Debug, Serialize)]
pub struct RoomPlayer {
    room_id: String,
    player: game::rooms::player::Player,
}

impl RoomPlayer {
    pub fn from_room(room: game::rooms::Room, player_id: Uuid) -> Self {
        Self {
            room_id: room.id().clone(),
            player: room.players().get(&player_id).cloned().unwrap(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RoomPlayerInfo {
    room_id: String,
    player_id: Uuid,
}

impl RoomPlayerInfo {
    pub fn new(room_id: String, player_id: Uuid) -> Self {
        Self {
            room_id,
            player_id,
        }
    }
}