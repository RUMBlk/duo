use std::{borrow::Borrow, hash::Hash};
use serde::Serialize;
use sea_orm::prelude::Uuid;
use tokio::sync::broadcast::Sender;
use crate::{ gateway::events::SharedTableEvents, gateway::payloads::Payload };

#[derive(Debug, Clone, Serialize)]
pub struct Player { //Структура, що описує гравця у кімнаті
    pub id: Uuid, //Ідентифікатор
    #[serde(skip)]
    pub sender: Sender<String>, //Надсилач
    pub is_ready: bool, //Готовність
    pub points: u64, //Очки
}

impl Player {
    pub fn new(id: Uuid, sender: Sender<String>) -> Self { //Конструктор
        Self { id, sender, is_ready: false, points: 0 }
    }
}

impl SharedTableEvents for Player { //Реалізація ознки SharedTableEvents для гравця
    //ця ознака надсилає зміни у будь-якому об'єкту таблиці усім її об'єктам
    fn insert(&self, other: Self) {
        let content = Payload::RoomPlayerNew(other).to_json_string(); //формування Payload
        let _ = self.sender.send(content); //Надсилання
    }

    fn update(&self, other: Self) {
        let content = Payload::RoomPlayerUpdate(other).to_json_string();
        let _ = self.sender.send(content);
    }

    fn delete(&self, other: Self) {
        let content = Payload::RoomPlayerLeft(other.id).to_json_string();
        let _ = self.sender.send(content);
    }
}

impl Eq for Player {} //Eq для Player за замовченням

impl PartialEq for Player { //PartialEq для Player, порівняння тільки за полем id
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Player { //Хешування за полем id
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Borrow<Uuid> for Player { //повернути поле id при запиті показника типу Uuid 
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}