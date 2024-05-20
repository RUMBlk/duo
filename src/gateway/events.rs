
use sea_orm::{prelude::Uuid, DatabaseConnection };
use std::sync::Arc;
use crate::database::queries;
use tokio::sync::{ broadcast::Sender, RwLock };
use super::payloads::*;

//Receive

pub async fn identify(
    db: &DatabaseConnection,
    payload: Identify,
    players: &Arc<RwLock<super::sessions::Table>>,
    sender: Sender<String>,
    store_in: &mut Option<Uuid>,
) -> Result<Payload, Error> {
    let token = Uuid::parse_str(payload.token().as_str()).map_err(|_| Error::BadToken)?;
    let uuid = queries::sessions::get_account_uuid(token).one(db).await
        .map_err(|_| Error::InternalServerError)?
        .ok_or(Error::InvalidToken)?;

    let mut players = players.write().await;
    let player = if let Some(player) = players.get(&uuid) {
        let mut player = player.write().await;
        player.set_sender(sender);
        player.clone()
    } else {
        let account = queries::accounts::by_uuid(uuid).one(db).await
            .map_err(|_| Error::InternalServerError)?
            .ok_or(Error::InvalidToken)?;
        let player = super::sessions::User::from_account(account, sender);
        players.insert(uuid, Arc::new(RwLock::new(player.clone())));
        player

    };
    *store_in = Some(player.uuid().clone());
    Ok(Payload::Ready(player))
}

/*
pub async fn send_to_room_players<'a>(
    players_ptr: Arc<RwLock<super::sessions::Table>>,
    room_player_ids: Keys<'a, Uuid, rooms::Player>,
    payload: Payload,
) {
    for player_id in room_player_ids {
        if let Some(player) = players_ptr.read().await.get(player_id) {
            let mut player = player.write().await;
            if let Some(sender) = &player.sender {
                if let Err(_) = sender.send(payload.to_json_string()) {
                    player.sender = None;
                }
            }
        }
    }
}
*/
/*
pub async fn room_create(room: RoomCreate, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>, sender: Sender<String>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let mut room = room.create_room(identity.uuid())?;
    let _ = room.join(room.password().clone(), identity.uuid(), sender);
    let id = room.generate_id().clone();
    rooms.insert(id.clone(), room.clone());
    let payload = RoomCreate::from_room(id, room.clone());
    Ok(Payload::RoomCreate(payload))
}

pub async fn room_update(payload: RoomUpdate, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(payload.id()).ok_or(Error::NotFound)?;
    if room.contains_player(identity.uuid()) == false {
        return Err(Error::Forbidden)
    }
    let result = room.batch_update(payload.room());
    if result.is_empty() { Ok(Payload::OK) } else { Err(Error::Room(result)) }
}

pub async fn room_join(payload: RoomJoin, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>,  sender: Sender<String>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(payload.id()).ok_or(Error::NotFound)?;
    room.join( payload.password().clone(), identity.uuid(), sender)
        .map_err(|_| Error::Forbidden)?;
    let new_player = RoomNewPlayer::new(payload.id().clone(), identity.uuid());
    let payload = RoomCreate::from_room(payload.id().clone(), room.clone());
    room.announce(Payload::RoomNewPlayer(new_player).to_json_string());
    Ok(Payload::RoomCreate(payload))
}

pub async fn room_leave(room_id: String, identity: &Option<Identity>, rooms: &Arc<RwLock<HashMap<String, Room>>>) -> Result<Payload, Error> {
    let identity = identity.clone().ok_or(Error::Forbidden)?;
    let mut rooms = rooms.write().await;
    let room = rooms.get_mut(&room_id).ok_or(Error::NotFound)?;
    room.leave(identity.uuid())
        .map_err(|_| Error::Forbidden)?;
    let payload = RoomPlayerLeft::new(room_id, identity.uuid());
    let _ = room.announce(Payload::RoomPlayerLeft(payload).to_json_string());
    Ok(Payload::OK)
}*/