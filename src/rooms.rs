use poem::{
    handler, http::StatusCode, web::{
        Data, Form, Json, Path
    }, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, sea_query::table, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set, TryInsertResult};
use std::{collections::HashMap, ops::Deref, sync::{ Arc, RwLock }, };
use super::database::entities;
use entities::prelude as tables;
use random_string;

#[derive(Serialize, Deserialize)]
pub struct Room {
    public: bool,
    password: Option<String>,
    owner: usize,
    max_players: u8,
    players: Vec<Uuid>
}

impl Room {
    pub fn new(public: bool, password: Option<String>, owner: Uuid, max_players: u8) -> Self {
        Self { public, password, owner: 0, max_players, players: vec![owner] }
    }
}

#[derive(Deserialize)]
struct CreateRoom {
    token: String,
    public: Option<bool>,
    password: Option<String>,
    max_players: u8,
}

#[handler]
pub async fn create_room(Form(request): Form<CreateRoom>, db: Data<&Arc<DatabaseConnection>>, rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let response = match tables::Sessions::find()
    .filter(entities::sessions::Column::Token.eq(request.token))
    .select_only()
    .column(entities::sessions::Column::Account)
    .into_tuple::<i64>()
    .one(db)
    .await {
        Ok(id) => {
            match id {
                Some(id) => {
                    match tables::Accounts::find_by_id(id).one(db).await {
                        Ok(account) => {
                            match account {
                                Some(account) => {
                                    let mut rooms = rooms.write().unwrap();
                                    let room = Room::new(request.public.unwrap_or(false), request.password, account.uuid, request.max_players);
                                    let room_json = serde_json::to_string(&room).expect("Failed to serialize room data");
                                    rooms.insert(random_string::generate(6, "0123456789XE"), room);
                                    drop(rooms);
                                    Ok(room_json)
                                },
                                None => Err(StatusCode::NOT_FOUND),
                            }
                        },
                        Err(_) => Err(StatusCode::BAD_GATEWAY),
                    }
                },
                None => Err(StatusCode::FORBIDDEN),
            }
        },
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    };

    let status = match response {
        Ok(_) => StatusCode::OK,
        Err(e) => e, 
    };

    Ok(
        Response::builder()
        .status(status)
        .body(response.unwrap_or_default())
        .set_content_type("application/json")
    )
}

#[derive(Deserialize)]
struct JoinRoom {
    token: String,
    room: String,
    password: Option<String>,
}

#[handler]
pub async fn join_room(Form(request): Form<JoinRoom>, db: Data<&Arc<DatabaseConnection>>, rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let response = match tables::Sessions::find()
    .filter(entities::sessions::Column::Token.eq(request.token))
    .select_only()
    .column(entities::sessions::Column::Account)
    .into_tuple::<i64>()
    .one(db)
    .await {
        Ok(id) => {
            match id {
                Some(id) => {
                    match tables::Accounts::find_by_id(id).one(db).await {
                        Ok(account) => {
                            match account {
                                Some(account) => {
                                    let mut rooms = rooms.write().unwrap();
                                    let result = match rooms.get_mut(&request.room) {
                                        Some(room) => {
                                            room.players.push(account.uuid);
                                            Ok(serde_json::to_string(&room).expect("Failed to serialize room data"))
                                        },
                                        None => Err(StatusCode::NOT_FOUND),
                                    };
                                    drop(rooms);
                                    result
                                },
                                None => Err(StatusCode::NOT_FOUND),
                            }
                        },
                        Err(_) => Err(StatusCode::BAD_GATEWAY),
                    }
                },
                None => Err(StatusCode::FORBIDDEN),
            }
        },
        Err(_) => Err(StatusCode::BAD_GATEWAY),
    };

    let status = match response {
        Ok(_) => StatusCode::OK,
        Err(e) => e, 
    };

    Ok(
        Response::builder()
        .status(status)
        .body(response.unwrap_or_default())
        .set_content_type("application/json")
    )
}