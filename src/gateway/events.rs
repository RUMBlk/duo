
use sea_orm::{prelude::Uuid, DatabaseConnection };
use std::sync::Arc;
use crate::database::queries;
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
    let uuid = queries::sessions::get_account_uuid(token).one(db).await
        .map_err(|_| Error::InternalServerError)?
        .ok_or(Error::InvalidToken)?;

    let mut players = players_ptr.write().await;
    let player = if let Some(player) = players.get(&uuid).cloned().as_mut() {
        player.set_sender(sender.clone());
        let mut rooms = rooms_ptr.write().await;
        if let Some(mut room) = player.room.as_ref().and_then(|room_id| rooms.get(room_id).cloned()) {
            if let Some(mut player) = room.players().get(&uuid).cloned() {
                player.sender = sender;
                let _ = player.sender.send(Payload::RoomCreate(room.clone()).to_json_string());
                room.players_mut().replace(player);
                rooms.replace(room);
            }
        }
        drop(rooms);
        player.to_owned()
    } else {
        let account = queries::accounts::by_uuid(uuid).one(db).await
            .map_err(|_| Error::InternalServerError)?
            .ok_or(Error::InvalidToken)?;
        let player = super::sessions::User::from_account(account, sender);
        player
    };
    players.replace(player.clone());
    *store_in = Some(player.uuid().clone());
    Ok(Payload::Ready(player.to_owned()))
}