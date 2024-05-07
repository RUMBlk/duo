use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::{collections::HashMap, ops::Deref, sync::{ Arc, RwLock }, };
use crate::database::queries;
use tokio::sync::broadcast;

use random_string;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    id: String,
    public: bool,
    password: Option<String>,
    owner: usize,
    max_players: u8,
    players: Vec<Uuid>,
    #[serde(skip)]
    broadcaster: Option<broadcast::Sender<String>>,
}

impl Room {
    pub fn new(public: bool, password: Option<String>, owner: Uuid, max_players: u8) -> Self {
        let id = random_string::generate(6, "0123456789XE");
        let broadcaster = Some(broadcast::channel(12).0);
        Self {id, broadcaster, public: public, password, owner: 0, max_players, players: vec![owner] }
    }

    pub fn set_owner(&mut self, id: usize) -> Result<(), ()> {
        if id >= self.players.len() { Err(()) }
        else { self.owner = id; Ok(()) }
    }
}

pub async fn room_env<F, T>(db: &DatabaseConnection, auth: Uuid, rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>, id: String, closure: F) -> Result<T, StatusCode>
where F: Fn(Uuid, &mut Room) -> Result<T, StatusCode>
{
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut rooms = rooms.write().unwrap();
    let room = rooms
        .get_mut(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = closure(player_id, room);
    if let Some(broadcaster) = &room.broadcaster {
        let _ = broadcaster.send(serde_json::to_string(&room).expect("Failed to serialize Room"));
    }
    if room.players.len() == 0 { rooms.remove(&id); }
    drop(rooms);
    result
}

#[handler]
pub async fn listener(
    Path(id): Path<String>,
    ws: WebSocket,
    rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>
) -> Result<impl IntoResponse, StatusCode> {
    let mut rooms = rooms.write().unwrap();
    let room = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let sender = room.broadcaster.as_ref().unwrap().clone();
    drop(rooms);
    let mut receiver = sender.subscribe();
    Ok(ws.on_upgrade(move |mut socket| async move {
        tokio::spawn(async move {
            while let Ok(room) = receiver.recv().await {
                let message = Message::Text(room);
                let _ = socket.send(message);
            }
        });
    }))
}

#[derive(Deserialize)]
struct RoomForm {
    public: Option<bool>,
    password: Option<String>,
    max_players: Option<u8>,
    owner: Option<usize>,
}

#[handler]
pub async fn create(
    main: &Request,
    req: Json<RoomForm>,
    db: Data<&Arc<DatabaseConnection>>,
    rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>
) -> Result<Json<Room>, StatusCode> {
    let auth = Uuid::parse_str(
        main.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut rooms = rooms.write().unwrap();
    let room = Room::new(req.public.unwrap_or(false), req.password.clone(), player_id, req.max_players.unwrap_or(2));
    let json = Json(room.clone());
    rooms.insert(room.id.clone(), room);
    drop(rooms);

    Ok(json)
}

#[handler]
pub async fn join(
    Path(id): Path<String>,
    main: &Request,
    req: Json<RoomForm>,
    db: Data<&Arc<DatabaseConnection>>,
    rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>
) -> Result<Response, StatusCode> {
    let auth = Uuid::parse_str(
        main.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    let json = room_env(db, auth, rooms, id, |player_id, room| {
        if req.password != room.password {
            return Err(StatusCode::FORBIDDEN);
        }

        room.players.push(player_id);
        let json = serde_json::to_string(&room)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        json
    }).await?;

    Ok(Response::builder()
        .body(json)
        .set_content_type("application/json")
    )
}

#[handler]
pub async fn leave(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>
) -> Result<StatusCode, StatusCode> {
    let auth = Uuid::parse_str(
        req.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    room_env(db, auth, rooms, id, |player_id, room| {
        room.players.retain(|item| *item != player_id);
        Ok(StatusCode::OK)
    }).await
}

#[handler]
pub async fn edit(Path(id): Path<String>, main: &Request, req: Json<RoomForm>, db: Data<&Arc<DatabaseConnection>>, rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>) -> Result<Json<Room>, StatusCode> {
    let auth = Uuid::parse_str(
        main.header("authorization")
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let db = db.deref().as_ref();
    room_env(db, auth, rooms, id, move |_player_id, room| {
        if let Some(public) = req.public { room.public = public; };
        room.password = req.password.clone();
        if let Some(max_players) = req.max_players { room.max_players = max_players; }
        if let Some(owner) = req.owner { let _ = room.set_owner(owner); }
        Ok(Json(room.deref().to_owned()))
    }).await
}