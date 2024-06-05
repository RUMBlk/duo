pub mod game;

use poem::{handler, http::StatusCode, web::{ self, Data, Json, Path }, Request, Response };
use sea_orm::{ prelude::Uuid, DatabaseConnection };
use serde::{Deserialize, Serialize};
use tokio::sync::{ RwLock, RwLockWriteGuard};
use std::{ ops::Deref, sync::Arc };
use crate::{ 
    Rooms,
    game::rooms::{self, Room, Partial},
    database::queries,
    gateway::sessions::User,
    runtime_storage::Table,
};

async fn prelude<'a>(
    db: &'a DatabaseConnection,
    auth: Option<&'a str>,
    players_ptr: &'a Arc<RwLock<crate::Players>>,
    rooms_ptr: &'a Arc<RwLock<Rooms>>
) -> Result<(RwLockWriteGuard<'a, crate::Players>, RwLockWriteGuard<'a, crate::Rooms>, User), StatusCode> {
    let auth = Uuid::parse_str(
        auth
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let players = players_ptr.write().await;
    let player = players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?.clone();

    let rooms = rooms_ptr.write().await;
    Ok((players, rooms, player))
}

fn limit() -> usize { 100 }

#[derive(Deserialize)]
struct RoomQuery {
    #[serde(default)]
    after: usize,
    #[serde(default = "limit")]
    limit: usize,
}

#[handler]
pub async fn get_rooms_list(query: web::Query<RoomQuery>, rooms: Data<&Arc<RwLock<Rooms>>>) -> Json<Vec<Partial>> {
    let rooms = rooms.read().await;
    let mut rooms_vec: Vec<Partial> = rooms
        .iter()
        .filter(|room| room.is_public)
        .skip(query.after)
        .take(query.limit)
        .map(|room| Partial(room.clone().into()))
        .collect();
    Json(rooms_vec)
}

#[derive(Deserialize)]
struct RoomCreate {
    name: String,
    is_public: bool,
    password: Option<String>,
    max_players: usize,
}

#[handler]
pub async fn create(
    req: &Request,
    body: Json<RoomCreate>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    if let Some(_) = &player.room {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut room = Room::create(body.name.clone(), body.is_public, body.password.clone(), *player.uuid(), body.max_players, player.sender.clone()).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    while let Some(_) = rooms.get(&room.clone()) {
        room.regenerate_id()
    };
    rooms.insert(room.clone());
    player.room = Some(room.id().clone());
    players.replace(player);
    Ok(Response::builder()
        .body(serde_json::to_string(&room).expect("Failed to serialize RoomResult")))

}

#[derive(Deserialize)]
struct RoomUpdate {
    name: Option<String>,
    is_public: Option<bool>,
    password: Option<String>,
    owner: Option<Uuid>,
    max_players: Option<usize>,
}


#[handler]
pub async fn update(
    Path(id): Path<String>,
    req: &Request,
    body: Json<RoomUpdate>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let mut room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?;

    if *room.owner() != *player.uuid() { return Err(StatusCode::FORBIDDEN) }

    let mut room = room.clone();
    let mut result = Vec::new();
    if let Some(ref value) = body.name { result.push(room.set_name(value.to_string())) }
    if let Some(value) = body.is_public { room.is_public = value; result.push(Ok(())) }
    if let Some(ref value) = body.password { result.push(room.set_password(Some(value.to_string()))) }
    if let Some(value) = body.owner { result.push(room.set_owner(value)) }
    if let Some(value) = body.max_players { result.push(room.set_max_players(value)) }
    for i in &result { 
            if let Err(_) = i { return Ok(
                Response::builder().status(StatusCode::BAD_REQUEST).body(
                    serde_json::to_string(&result).unwrap()
                )
            )
        }
    };
    rooms.replace(room);
    Ok(Response::builder().status(StatusCode::OK).body(""))

}

#[derive(Serialize, Deserialize)]
struct RoomJoin {
    password: Option<String>,
}

#[handler]
pub async fn join(
    Path(id): Path<String>,
    req: &Request,
    body: Json<RoomJoin>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Json<Room>, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone();
    if let Some(room_id) = &player.room {
        if id != *room_id {
            return Err(StatusCode::FORBIDDEN);
        } else {
            return Ok(Json(room))
        }
    }
    room.join(body.password.clone(), *player.uuid(), player.sender.clone()).await.map_err(|_| StatusCode::FORBIDDEN)?;
    player.room = Some(room.id().clone());
    players.replace(player);
    Ok(Json(room))
}


#[handler]
pub async fn leave(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let mut room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone();
    let leave = room.leave(player.uuid().clone()).await;
    if let Err(rooms::Error::CantAssignNewOwner) = leave {
        rooms.remove(&room.clone());
    } else if let Ok(true) = leave {
        rooms.replace(room);
    } else {
        leave.map_err(|_| StatusCode::FORBIDDEN)?;
    }
    player.room = None;
    players.replace(player);
    Ok(StatusCode::OK)
}

#[handler]
pub async fn ready(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let room = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    room.player_switch_ready(player.uuid().clone()).await.map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(StatusCode::OK)
}