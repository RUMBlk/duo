use poem::{handler, http::StatusCode, web::{ self, Data, Json, Path }, Request, Response };
use sea_orm::{ prelude::Uuid, DatabaseConnection };
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::{ sync::Arc, ops::Deref };
use crate::game::rooms::{self, ReturnCode, Room};
use crate::database::queries;
use crate::gateway;

fn n() -> usize { 100 }

#[derive(Deserialize)]
struct RoomQuery {
    #[serde(default)]
    i: usize,
    #[serde(default = "n")]
    n: usize,
    #[serde(default)]
    partial: bool,
}

#[derive(Serialize)]
struct RoomRow {
    id: String,
    #[serde(flatten)]
    room: Room,
    players: usize,
}

impl RoomRow {
    pub fn new(id: &String, room: &Room, partial: bool) -> Self {
        let processed_room = match partial {
            true => room.get_partial(),
            false => room.clone(),
        };
        Self { id: id.clone(), room: processed_room, players: room.number_of_players() }
    }
}

#[handler]
pub async fn get_rooms_list(query: web::Query<RoomQuery>, rooms: Data<&Arc<RwLock<rooms::Table>>>) -> Json<Vec<RoomRow>> {
    let rooms = rooms.read().await;
    let mut rooms_vec: Vec<RoomRow> = rooms
        .iter()
        .skip(query.i)
        .take(query.n)
        .map(|(id, room)| RoomRow::new(id, room, query.partial))
        .collect();
    Json(rooms_vec)
}

#[derive(Serialize)]
struct RoomResult {
    room: Option<Room>,
    errors: Vec<ReturnCode>
}

impl RoomResult {
    pub fn new(room: Option<Room>, errors: Vec<ReturnCode>) -> Self {
        Self {room, errors}
    }
}

#[handler]
pub async fn create(
    req: &Request,
    body: Json<Room>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<rooms::Table>>>,
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
    if let Some(_) = player.room { return Err(StatusCode::FORBIDDEN) };

    let mut rooms = rooms_ptr.write().await;
    let mut room = Room::default();
    let errors: Vec<ReturnCode> = room.batch_update(&body).into_iter().filter_map(|value| {
        match value {
            Ok(_) => None,
            Err(value) => Some(value),
        }
    }).collect();
    let room = if errors.len() == 0 {
        let _ = room.join(room.password().clone(), player_id);
        player.room = room.id().clone();
        let _ = room.set_owner(player_id);

        let id = loop {
            let id = room.id().clone().unwrap();
            if rooms.contains_key(&id) { room.regenerate_id(); } 
            else { break id; }
        };
        rooms.insert(id, room.clone());
        tokio::spawn(gateway::events::room_create(players_ptr.to_owned(), room.clone()));
        Some(room)
    } else { None };
    Ok(Response::builder()
        .body(serde_json::to_string(&RoomResult::new(room, errors)).expect("Failed to serialize RoomResult")))

}

#[handler]
pub async fn update(
    Path(id): Path<String>,
    req: &Request,
    body: Json<Room>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<rooms::Table>>>,
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
    let mut room = rooms.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    let errors: Vec<ReturnCode> = room.batch_update(&body).into_iter().filter_map(|value| {
        match value {
            Ok(_) => None,
            Err(value) => Some(value),
        }
    }).collect();

    let room = room.clone();
    tokio::spawn(gateway::events::room_update(players_ptr.to_owned(), room.clone()));
    Ok(Response::builder()
        .body(serde_json::to_string(&RoomResult::new(Some(room), errors)).expect("Failed to serialize RoomResult")))

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
    rooms_ptr: Data<&Arc<RwLock<rooms::Table>>>,
) -> Result<Json<Room>, StatusCode> {
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
    let mut room = rooms.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    room.join(body.password.clone(), player_id).map_err(|_| StatusCode::FORBIDDEN)?;
    player.room = Some(id);
    tokio::spawn(gateway::events::room_players_update(players_ptr.to_owned(), room.clone()));
    Ok(Json(room.clone()))

}

#[handler]
pub async fn leave(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<gateway::sessions::Table>>>,
    rooms_ptr: Data<&Arc<RwLock<rooms::Table>>>,
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
    let mut room = rooms.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    match room.leave(player_id).map_err(|_| StatusCode::PRECONDITION_FAILED)? {
        ReturnCode::OwnerChanged => { tokio::spawn(gateway::events::room_update(players_ptr.to_owned(), room.clone())); },
        _ => {},
    };
    player.room = None;
    tokio::spawn(gateway::events::room_players_update(players_ptr.to_owned(), room.clone()));
    Ok(StatusCode::OK)

}