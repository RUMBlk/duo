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
    RoomPlayerNew(game::rooms::player::Player),
    #[serde(skip_deserializing)]
    RoomPlayerUpdate(game::rooms::player::Player),
    #[serde(skip_deserializing)]
    RoomPlayerLeft(Uuid),
    #[serde(skip_deserializing)]
    RoomCreate(game::rooms::Room),
    #[serde(skip_deserializing)]
    RoomUpdate(game::rooms::Room),
    #[serde(skip_deserializing)]
    RoomDelete(String),
    #[serde(skip_deserializing)]
    GameStarted(game::gameplay::Game),
    #[serde(skip_deserializing)]
    GameNewTurn(game::gameplay::Game),
    #[serde(skip_deserializing)]
    GamePlayerCards(Vec<game::gameplay::card::Card>),
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