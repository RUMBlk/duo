pub mod reimpl;

use poem::{handler, http::StatusCode, web::{ self, Data, Json, Path }, Request, Response };
use sea_orm::{ prelude::Uuid, DatabaseConnection };
use serde::{Deserialize, Serialize};
use tokio::sync::{ RwLock, RwLockWriteGuard};
use std::{ ops::Deref, sync::Arc };
use crate::{game::rooms::Partial, Rooms};
use crate::game::rooms::Interaction;
use crate::database::queries;
use crate::gateway::sessions::User;

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
        .map_err(|_| StatusCode::BAD_GATEWAY)?
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
        .map(|room| Partial(room.clone()))
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

    let mut room = reimpl::Room::create(body.name.clone(), body.is_public, body.password.clone(), *player.uuid(), body.max_players, player.sender.clone())
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    while let Some(_) = rooms.get(&room.0) {
        room.0.regenerate_id()
    };
    rooms.replace(room.0.clone());
    player.room = Some(room.0.id().clone());
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

    let mut room = reimpl::Room(room.clone());
    let result = room.update(body.name.clone(), body.is_public, body.password.clone(), body.owner, body.max_players);
    rooms.replace(room.0);
    Ok(Response::builder()
        .body(serde_json::to_string(&result).expect("Failed to serialize RoomResult")))

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
) -> Result<Json<reimpl::Room>, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    if let Some(room) = &player.room {
        if id != *room {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    let mut room = reimpl::Room(rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone());
    room.join(body.password.clone(), *player.uuid(), player.sender.clone()).map_err(|_| StatusCode::FORBIDDEN)?;
    rooms.replace(room.0.clone());
    player.room = Some(room.0.id().clone());
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
    let mut room = reimpl::Room(rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone());
    room.leave(player.uuid().clone()).map_err(|_| StatusCode::FORBIDDEN)?;
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
    let mut room = reimpl::Room(rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone());
    room.player_switch_ready(player.uuid().clone()).map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(StatusCode::OK)
}