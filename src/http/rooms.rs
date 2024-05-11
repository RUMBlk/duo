use poem::{handler, web::{ self, Json, Data } };
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::game::rooms::{ Room, Table };

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
pub async fn get_rooms_list(query: web::Query<RoomQuery>, rooms: Data<&Arc<RwLock<Table>>>) -> Json<Vec<RoomRow>> {
    let rooms = rooms.read().await;
    let mut rooms_vec: Vec<RoomRow> = rooms
        .iter()
        .skip(query.i)
        .take(query.n)
        .map(|(id, room)| RoomRow::new(id, room, query.partial))
        .collect();
    Json(rooms_vec)
}
