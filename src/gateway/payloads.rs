use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::prelude::Uuid;
use crate::game::rooms::{self, Room};
use sea_orm::prelude::DateTimeWithTimeZone;
use crate::database::entities;

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
    //From Server
    OK,
    Error(Error),
    Hello(Hello),
    RoomNewPlayer(RoomNewPlayer),
    RoomPlayerLeft(RoomPlayerLeft),
    //From Server/Client
    Identify(Identify),
    Identity(Identity),
    RoomCreate(RoomCreate),
    RoomUpdate(RoomUpdate),
    //From Client
    RoomJoin(RoomJoin),
    RoomLeave(String),
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
        Room(Vec<Result<rooms::ReturnCode, rooms::ReturnCode>>)
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
pub struct RoomCreate {
    default_where_error: Option<bool>,
    id: Option<String>,
    #[serde(flatten)]
    room: Room,
}

impl RoomCreate {
    pub fn from_room(id: String, room: Room) -> Self {
        Self {
            default_where_error: None,
            id: Some(id),
            room,
        }
    }

    pub fn create_room(&self, owner: Uuid) -> Result<Room, Error> {
        let mut room = Room::default();
        let errors = room.batch_update(&self.room);
        let _ = room.set_owner(owner);
        if self.default_where_error == Some(true) || errors.len() == 0 {
            Ok(room)
        } else {
            Err(Error::Room(errors))
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomUpdate {
    id: String,
    #[serde(flatten)]
    room: Room,
}

impl RoomUpdate {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn room(&self) -> &Room {
        &self.room
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomJoin {
    id: String,
    password: Option<String>,
}

impl RoomJoin {
    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomNewPlayer {
    room_id: String,
    player_id: Uuid,
}

impl RoomNewPlayer {
    pub fn new(room_id: String, player_id: Uuid) -> Self {
        Self {
            room_id,
            player_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomPlayerLeft {
    room_id: String,
    player_id: Uuid,
}

impl RoomPlayerLeft {
    pub fn new(room_id: String, player_id: Uuid) -> Self {
        Self {
            room_id,
            player_id,
        }
    }
}