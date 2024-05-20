pub mod reimpl;
pub mod player;

use poem::{handler, http::StatusCode, web::{ self, Data, Json, Path }, Request, Response };
use sea_orm::{ prelude::Uuid, DatabaseConnection };
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::{ ops::Deref, sync::Arc };
use crate::Rooms;
use crate::game::rooms::Interaction;
use crate::database::queries;
use crate::gateway;

fn limit() -> usize { 100 }

#[derive(Deserialize)]
struct RoomQuery {
    #[serde(default)]
    after: usize,
    #[serde(default = "limit")]
    limit: usize,
}

#[handler]
pub async fn get_rooms_list(query: web::Query<RoomQuery>, rooms: Data<&Arc<RwLock<Rooms>>>) -> Json<Vec<reimpl::RoomPartial>> {
    let rooms = rooms.read().await;
    let mut rooms_vec: Vec<reimpl::RoomPartial> = rooms
        .iter()
        .skip(query.after)
        .take(query.limit)
        .map(|room| reimpl::RoomPartial(room.clone()))
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
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let auth = Uuid::parse_str(
        req.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut players = players_ptr.read().await;
    let mut player = players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?.write().await;

    let mut rooms = rooms_ptr.write().await;
    let mut room = reimpl::Room::create(body.name.clone(), body.is_public, body.password.clone(), player.to_owned(), body.max_players)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    while let Some(_) = rooms.get(&room.0) {
        room.0.regenerate_id()
    };
    rooms.insert(room.0.clone());
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
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let auth = Uuid::parse_str(
        req.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut players = players_ptr.write().await;
    players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?;

    let mut rooms = rooms_ptr.write().await;
    let mut room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?;

    if *room.owner() != Some(player_id) { return Err(StatusCode::FORBIDDEN) }

    let mut room = reimpl::Room(room.clone());
    let result = room.update(body.name.clone(), body.is_public, body.password.clone(), body.owner, body.max_players);
    rooms.insert(room.0);
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
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Json<reimpl::Room>, StatusCode> {
    let auth = Uuid::parse_str(
        req.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut players = players_ptr.write().await;
    let mut player = players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?.write().await;

    let mut rooms = rooms_ptr.write().await;
    let mut room = reimpl::Room(rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone());
    room.join(body.password.clone(), player.clone().into()).map_err(|_| StatusCode::FORBIDDEN)?;
    rooms.insert(room.0.clone());
    Ok(Json(room))
}


#[handler]
pub async fn leave(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let auth = Uuid::parse_str(
        req.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut players = players_ptr.write().await;
    let mut player = players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?.write().await;

    let mut rooms = rooms_ptr.write().await;
    let mut room = reimpl::Room(rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone());
    room.leave(player.clone().into()).map_err(|_| StatusCode::FORBIDDEN)?;
    if room.0.players().len() > 0 {
        rooms.insert(room.0.clone());
    } else {
        rooms.remove(&room.0);
    }
    Ok(StatusCode::OK)
}