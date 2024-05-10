use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ self, websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::{DateTimeWithTimeZone, Uuid}, DatabaseConnection, Iden};
use std::{collections::HashMap, ops::Deref, sync::Arc, };
use crate::{auth, database::{entities, queries}, game::room::{self, Room}};
use tokio::sync::{ broadcast::{self, Receiver, Sender}, RwLock };
use futures_util::StreamExt;
use sea_orm::prelude::DateTime;

use random_string;

use super::payloads::*;

pub async fn identify(db: &DatabaseConnection, payload: Identify, store_in: &mut Option<Identity>) -> Result<Payload, Error> {
    let token = Uuid::parse_str(payload.token().as_str()).map_err(|_| Error::BadToken)?;
    let uuid = queries::sessions::get_account_uuid(token).one(db).await
        .map_err(|_| Error::InternalServerError)?
        .ok_or(Error::InvalidToken)?;
    let account = queries::accounts::by_uuid(uuid).one(db).await
        .map_err(|_| Error::InternalServerError)?
        .ok_or(Error::InvalidToken)?;
    let identity = Identity::from(account);
    *store_in = Some(identity.clone());
    Ok(Payload::Identity(identity))
}

pub async fn room_create(mut payload: Room, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>, sender: Sender<String>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let _ = payload.join(payload.password().clone(), identity.uuid(), sender);
    let _ = payload.set_owner(identity.uuid());
    let room_id = payload.generate_id().clone();
    rooms.insert(room_id.clone(), payload.clone());
    Ok(Payload::RoomCreate(room_id, payload))
}

pub async fn room_update(room_id: String, mut payload: RoomUpdate, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>) -> Result<Payload, Error> {
    identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(&room_id).ok_or(Error::NotFound)?;
    let result = payload.apply(room);
    room.announce(Payload::RoomUpdate(room_id, payload).to_json_string());
    Ok(Payload::RoomUpdateResult(result))
}

pub async fn room_join(payload: RoomJoin, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>,  sender: Sender<String>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(&payload.room_id()).ok_or(Error::NotFound)?;
    room.join( payload.password(), identity.uuid(), sender)
        .map_err(|_| Error::Forbidden)?;
    room.announce(Payload::RoomJoined(identity.uuid()).to_json_string());
    Ok(Payload::RoomCreateWithPlayers(payload.room_id(), room.clone(), room.players()))
}

pub async fn room_leave(room_id: String, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(&room_id).ok_or(Error::NotFound)?;
    room.leave(identity.uuid())
        .map_err(|_| Error::Forbidden)?;
    let _ = room.announce(Payload::RoomLeft(identity.uuid()).to_json_string());
    Ok(Payload::OK)
}