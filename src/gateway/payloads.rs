use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ self, websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection, Iden};
use std::{collections::HashMap, ops::Deref };
use crate::{auth, database::queries, game::room::{self, Room}};
use tokio::sync::broadcast;
use futures_util::StreamExt;
use sea_orm::prelude::DateTime;
use sea_orm::prelude::DateTimeWithTimeZone;
use crate::database::entities;

use random_string;

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
    OK,
    Error(Error),
    #[serde(skip_deserializing)]
    Hello(Hello),
    Identify(Identify),
    Identity(Identity),
    RoomCreate(String, Room),
    #[serde(skip_deserializing)]
    RoomCreateWithPlayers(String, Room, room::Players),
    #[serde(skip_deserializing)]
    RoomUpdate(String, RoomUpdate),
    RoomUpdateResult(RoomUpdateResult),
    RoomJoin(RoomJoin),
    RoomJoined(Uuid),
    RoomLeave(String),
    RoomLeft(Uuid),
}

impl Payload {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize Gateway Payload")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
        BadRequest,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    uuid: Uuid,
    login: String,
    display_name: String,
    created_at: DateTimeWithTimeZone,
}

impl Identity {
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl From<entities::accounts::Model> for Identity {
    fn from(model: entities::accounts::Model) -> Self {
        let uuid = model.uuid;
        let login = model.login;
        let display_name = model.display_name;
        let created_at = model.created_at;
        Self { uuid, login, display_name, created_at }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomUpdate {
    name: Option<String>,
    public: Option<bool>,
    password: Option<String>,
    owner: Option<Uuid>,
    max_players: Option<usize>,
}

impl RoomUpdate {
    pub fn name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn public(&self) -> Option<bool> {
        self.public
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }

    pub fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub fn max_players(&self) -> Option<usize> {
        self.max_players
    }

    pub fn apply(&self, room: &mut Room) -> RoomUpdateResult {
        let name = self.name.as_ref().and_then(|name| {
            Some(room.set_name(name.clone()))
        });
        let public = self.public.and_then(|public| {
            room.public = public;
            Some(Ok(room::ReturnCode::OK))
        });
        let password = self.password.as_ref().and_then(|password| {
            let password = if password.is_empty() {
                None
            } else { Some(password) };
            Some(room.set_password(password.cloned()))
        });
        let owner = self.owner.and_then(|player_id| {
            Some(room.set_owner(player_id))
        });
        let max_players = self.max_players.and_then(|max_players| {
            Some(room.set_max_players(max_players))
        });
        RoomUpdateResult::new(name, public, password, owner, max_players)
    }
}

type RoomResult = Option<Result<room::ReturnCode, room::ReturnCode>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomUpdateResult {
    name: RoomResult,
    public: RoomResult,
    password: RoomResult,
    owner: RoomResult,
    max_players: RoomResult,
}

impl RoomUpdateResult {
    fn new(
        name: RoomResult,
        public: RoomResult,
        password: RoomResult,
        owner: RoomResult,
        max_players: RoomResult,
    ) -> Self {
        Self { name, public, password, owner, max_players }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomJoin {
    room_id: String,
    password: Option<String>,
}

impl RoomJoin {
    pub fn room_id(&self) -> String {
        self.room_id.clone()
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }
}