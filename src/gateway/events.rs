
use sea_orm::{prelude::Uuid, DatabaseConnection };
use std::sync::Arc;
use crate::{ database::queries, runtime_storage::SharedTable };
use tokio::sync::{ broadcast::Sender, RwLock };
use super::payloads::*;

//Receive

pub async fn identify(
    db: &DatabaseConnection,
    payload: Identify,
    players_ptr: &Arc<RwLock<crate::Players>>,
    rooms_ptr: &Arc<RwLock<crate::Rooms>>,
    sender: Sender<String>,
    store_in: &mut Option<Uuid>,
) -> Result<Payload, Error> {
    let token = Uuid::parse_str(payload.token().as_str()).map_err(|_| Error::BadToken)?;
    let uuid = queries::sessions::handle(db, token).await
        .map_err(|_| Error::InternalServerError)?;

    let mut players = players_ptr.write().await;
    let player = if let Some(player) = players.get(&uuid).cloned().as_mut() {
        player.set_sender(sender.clone());
        let rooms = rooms_ptr.read().await;
        if let Some(room) = player.room.as_ref().and_then(|room_id| rooms.get(room_id).cloned()) {
            let _ = room.players().write().await.shared_update(&uuid, |player| {
                player.sender = sender.clone();
                Ok::<(), ()>(())
            });
            let _ = sender.send(Payload::RoomCreate(room.clone()).to_json_string());
        }
        drop(rooms);
        player.to_owned()
    } else {
        let account = queries::accounts::by_uuid(uuid).one(db).await
            .map_err(|_| Error::InvalidToken)?
            .ok_or(Error::InvalidToken)?;
        let player = super::sessions::User::from_account(account, sender);
        player
    };
    players.replace(player.clone());
    *store_in = Some(player.uuid().clone());
    Ok(Payload::Ready(player.to_owned()))
}

pub trait TableEvents {
    fn insert(&self);
    fn update(&self);
    fn delete(&self);
}

pub trait SharedTableEvents {
    fn insert(&self, other: Self);
    fn update(&self, other: Self);
    fn delete(&self, other: Self);
}