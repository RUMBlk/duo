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
    RoomPlayersUpdate(game::rooms::Players),
    //From Server/Client
    Identify(Identify),
    Ready(super::sessions::User),
    RoomCreate(game::rooms::Room),
    RoomUpdate(game::rooms::Room),
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