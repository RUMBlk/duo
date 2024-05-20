use sea_orm::prelude::Uuid;
use serde::{ Serialize, Deserialize, ser::{ self, SerializeStruct } };
use serde_json;
use crate::{game, http};
use crate::http::rooms::reimpl::*;

use super::sessions::{self, User};

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
    RoomCreate(Room),
    #[serde(skip_deserializing)]
    RoomUpdate(Room),
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

#[derive(Debug)]
pub struct Player(game::rooms::Player<http::rooms::player::Data>);
impl ser::Serialize for Player {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: ser::Serializer {
        let mut state = serializer.serialize_struct("player", 3)?;
        state.serialize_field("data", &self.0.data)?;
        state.serialize_field("is_ready", &self.0.is_ready)?;
        state.serialize_field("points", &self.0.points)?;
        state.end()
    }
}

impl From<game::rooms::Player<http::rooms::player::Data>> for Player {
    fn from(value: game::rooms::Player<http::rooms::player::Data>) -> Self {
        Self { 0: value }
    }
}

#[derive(Debug, Serialize)]
pub struct RoomPlayer {
    room_id: String,
    player: Player,
}

impl RoomPlayer {
    pub fn from_room(room: crate::Room, player_query: http::rooms::player::Data) -> Self {
        Self {
            room_id: room.id().clone(),
            player: Player::from(room.players().get::<http::rooms::player::Data>(&player_query).cloned().unwrap()),
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