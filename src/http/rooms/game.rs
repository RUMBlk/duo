use poem::{handler, http::StatusCode, web::{ Data, Path }, Request, Response };
use sea_orm::{prelude::DatabaseConnection, Set};
use serde::Deserialize;
use tokio::sync::RwLock;
use std::{ ops::Deref, sync::Arc };
use crate::{ 
    Rooms,
    runtime_storage::Table,
    game::gameplay::Ok,
    database::queries::accounts,
};
use super::prelude;
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
    let (_players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let room = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    match room.play_game(*player.uuid(), card_id).await.map_err(|_e| { StatusCode::PRECONDITION_FAILED } )? {
        Ok::GameOver(players) => {
            //Implement upload to the Database
            for (index, player) in players.iter().enumerate() {
                let _ = accounts::update(db, player.id().clone(), |values, account| {
                    account.games_played = Set(values.games_played + 1);
                    if index <= players.len() / 2 {
                        account.wins = Set(values.wins + 1);    
                    } else {
                        account.loses = Set(values.loses + 1);
                    }
                    account.cards_had = Set(values.cards_had + *player.cards_had() as i64);
                    account.points = Set(values.points + *player.points() as i64);
                    account.max_points = Set(values.max_points.max(*player.points() as i16))
                }).await;
            }
        },
        _ => {},
    }
    Ok(StatusCode::OK)
}