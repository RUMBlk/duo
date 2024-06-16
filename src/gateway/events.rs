
use sea_orm::{prelude::Uuid, DatabaseConnection };
use std::sync::Arc;
use crate::database::queries;
use tokio::sync::{ broadcast::Sender, RwLock };
use super::payloads::*;

//Receive

pub async fn identify( //Функція яка ідентифікує акаунт за токеном, авторизує та відновлює сесію
    db: &DatabaseConnection,
    payload: Identify,
    players_ptr: &Arc<RwLock<crate::Players>>,
    rooms_ptr: &Arc<RwLock<crate::Rooms>>,
    sender: Sender<String>,
    store_in: &mut Option<Uuid>,
) -> Result<Payload, Error> {
    let token = Uuid::parse_str(payload.token().as_str()).map_err(|_| Error::BadToken)?; //Парсинг токену в Uuid
    let uuid = queries::sessions::handle(db, token).await
        .map_err(|_| Error::InvalidToken)?; //Перевірка токену на валідність та доставання id акаунту, повернення помилки у разі невірного токену

    let mut players = players_ptr.write().await; //замок на таблицю гравців
    let player = if let Some(player) = players.get(&uuid).cloned().as_mut() { //якщо гравець вже авторизований
        player.set_sender(sender.clone()); //назначити новий надсилач
        let rooms = rooms_ptr.read().await; //замок на таблицю кімнат
        //якщо кімната до якої був приєднаний гравець до виходу все ще існує
        if let Some(room) = player.room.as_ref().and_then(|room_id| rooms.get(room_id).cloned()) {
            let _ = room.player_update_sender(uuid, sender.clone()).await; //поновити сесію в кімнаті
        }
        drop(rooms); //відімкнути таблицю кімнат
        player.to_owned() //клонування гравця з таблиці та повернути у якості результату замикання
    } else { //інакше
        let account = queries::accounts::by_uuid(uuid).one(db).await
            .map_err(|_| Error::InternalServerError)?
            .ok_or(Error::InvalidToken)?; //Дістати акаунт за Uuid
        let player = super::sessions::User::from_account(account, sender); //та перетворити у сесію
        player
    };
    players.replace(player.clone()); //Помістити нове значення у таблицю, замінивши старе якщо воно є
    *store_in = Some(player.uuid().clone()); //зберегти id у змінній за вказівником
    Ok(Payload::Ready(player.to_owned()))
}

pub trait TableEvents { //Ознака яка задає методи для надсилання подій всередині об'єкту
    fn insert(&self);
    fn update(&self);
    fn delete(&self);
}

pub trait SharedTableEvents { //Ознака яка задає методи для надсилання подій про об'єкт таблиці усім її об'єктам
    fn insert(&self, other: Self);
    fn update(&self, other: Self);
    fn delete(&self, other: Self);
}