use serde::Serialize;
use sea_orm::prelude::Uuid;
use std::{
    hash::{Hash, Hasher},
    borrow::Borrow,
};
use tokio::sync::broadcast::Sender;
use crate::database::entities;

#[derive(Debug, Clone, Serialize)]
pub struct User { //Структура, яка описує сесію користувача
    #[serde(skip)]
    pub sender: Sender<String>, //надсилач
    uuid: Uuid, //id гравця
    pub room: Option<String>, //кімната до якої гравець приєднався
}

impl User {
    pub fn from_account(account: entities::accounts::Model, sender: Sender<String>) -> Self { //конструктор з рядку БД та надсилача
        Self { 
            sender,
            uuid: account.uuid,
            room: None,
        }
    }

    pub fn set_sender(&mut self, sender: Sender<String>) { //Геттер надсилача
        self.sender = sender;
    }

    pub fn uuid(&self) -> &Uuid { //Геттер uuid
        &self.uuid
    }
}

impl Eq for User { } //Реалізація Eq для User за замовченням

impl PartialEq for User { //Перевизначення порівнювання на порівнювання за полем uuid
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Hash for User { //Реалізація хешування за полем uuid
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl Borrow<Uuid> for User { //Реалізація повернення поля uuid у якості показника типу Uuid для User
    fn borrow(&self) -> &Uuid {
        &self.uuid
    }
}