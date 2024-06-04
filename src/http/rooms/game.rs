use poem::{handler, http::StatusCode, web::{ Data, Path }, Request, Response };
use sea_orm::prelude::DatabaseConnection;
use serde::Deserialize;
use tokio::sync::RwLock;
use std::{ ops::Deref, sync::Arc };
use crate::Rooms;
use super::prelude;
use crate::runtime_storage::Table;
use futures::executor;

#[handler]
pub async fn get(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, rooms, _player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let game = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?.game.as_ref().ok_or(StatusCode::NO_CONTENT)?.read().await.clone();
    Ok(Response::builder().body(serde_json::to_string(&game).unwrap()))
}

#[handler]
pub async fn start(
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, mut rooms, player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    rooms.update(&id, |room| {
        if player.uuid() == room.owner() {
            executor::block_on(room.start_game()).map_err(|_| StatusCode::CONFLICT)
        } else {
            Err(StatusCode::FORBIDDEN)
        }?;
        Ok::<(), StatusCode>(())
    })?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct Play {
    id: String,
    card_id: Option<usize>,
}

#[handler]
pub async fn play(
    Path(Play { id, card_id }): Path<Play>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, rooms, player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let mut game = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?.game.as_ref().ok_or(StatusCode::NO_CONTENT)?.write().await;
    game.play(*player.uuid(), card_id).map_err(|e| { eprintln!("{:?}", e); StatusCode::PRECONDITION_FAILED } )?;
    Ok(StatusCode::OK)
}