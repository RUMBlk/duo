use sea_orm::prelude::Uuid;
use serde::{ Serialize, Deserialize };
use serde_json;
use crate::game::{self, gameplay::player::Losers};

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload { //Список усих можливих подій
    //From Server
    #[serde(skip_deserializing)] //Пропуск десеріалізації
    Error(Error), //Подія помилки
    #[serde(skip_deserializing)]
    Hello(Hello), //Подія привітання
    #[serde(skip_deserializing)]
    RoomPlayerNew(game::rooms::player::Player), //Подія приєднання гравця до кімнати
    #[serde(skip_deserializing)]
    RoomPlayerUpdate(game::rooms::player::Player), //Подія оновлення гравця кімнати
    #[serde(skip_deserializing)]
    RoomPlayerLeft(Uuid), //Подія виходу гравця з кімнати
    #[serde(skip_deserializing)]
    RoomCreate(game::rooms::Room), //Подія створення нової кімнати
    #[serde(skip_deserializing)]
    RoomUpdate(game::rooms::Room), //Подія оновлення кімнати
    #[serde(skip_deserializing)]
    RoomDelete(String), //Подія видалення кімнати
    #[serde(skip_deserializing)]
    GameStarted(game::gameplay::Game), //Подія створення нової гри
    #[serde(skip_deserializing)]
    GameNewTurn(game::gameplay::Game), //Подія нового ходу гри
    #[serde(skip_deserializing)]
    GamePlayerCards(Vec<game::gameplay::card::Card>), //Подія оголошення карт відповідного гравця
    #[serde(skip_deserializing)]
    GameOver(Losers), //Подія закінчення гри
    //From Server/Client
    Identify(Identify), //Подія ідентифікації та авторизації за токеном
    #[serde(skip_deserializing)]
    Ready(super::sessions::User), //Подія окінчення ідентифікації
    /*//From Client
    RoomJoin(RoomJoin),
    RoomLeave(String),*/
}

impl Payload {
    pub fn to_json_string(&self) -> String { //серіалізація у рядок
        serde_json::to_string(self).expect("Failed to serialize Gateway Payload")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error { //Список помилок
        BadRequest(String),
        Declined,
        BadToken,
        InvalidToken,
        InternalServerError,
        NotFound,
        Forbidden,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hello { //Структура, що описує подію привітання
    heartbeat_interval: u64, 
}

impl Hello {
    pub fn new(heartbeat_interval: u64) -> Self {
        Self { heartbeat_interval }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identify { //Структура, що описує подію ідентифікації
    token: String,
}

impl Identify {
    pub fn token(&self) -> String { //Геттер token
        self.token.clone()
    }
}