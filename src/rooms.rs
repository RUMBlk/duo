use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::{collections::HashMap, ops::Deref, sync::Arc, vec, };
use crate::database::queries;
use tokio::sync::{ broadcast, RwLock };

use random_string;

use super::game::room::Room;

pub async fn room_env<F, T>(db: &DatabaseConnection, auth: Uuid, rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>, id: String, closure: F) -> Result<T, StatusCode>
where F: Fn(Uuid, &mut Room) -> Result<T, StatusCode>
{
    let player_id = queries::sessions::get_account_uuid(auth).one(db).await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::FORBIDDEN)?;

    let mut rooms = rooms.write().await;
    let room = rooms
        .get_mut(&id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = closure(player_id, room);
    /*if let Some(broadcaster) = &room.broadcaster {
        let _ = broadcaster.send(serde_json::to_string(&room).expect("Failed to serialize Room"));
    }*/
    drop(rooms);
    result
}

#[derive(Deserialize)]
struct RoomForm {
    public: Option<bool>,
    name: Option<String>,
    password: Option<String>,
    max_players: Option<u8>,
    owner: Option<usize>,
}

/*
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
        //if let Some(owner) = req.owner { let _ = room.set_owner(owner); }
        Ok(Json(room.deref().to_owned()))
    }).await
}
*/