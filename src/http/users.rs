use poem::{
    handler, http::StatusCode, web::{
        Data, Json, Path
    },
    Response,
};
use sea_orm::DatabaseConnection;
use std::{ops::Deref, sync::Arc};
use crate::database::{self, entities::accounts, queries};
use serde::{ser::SerializeStruct, Serialize};

struct User(pub database::entities::accounts::Model);

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("User", 3)?;
        state.serialize_field("uuid", &self.0.uuid)?;
        state.serialize_field("login", &self.0.login)?;
        state.serialize_field("display_name", &self.0.display_name)?;
        state.serialize_field("created_at", &self.0.created_at)?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize)]
struct UserStat {
    games_played: i64,
    points: i64,
    cards_had: i64,
    wins: i32,
    loses: i32,
    max_points: i16,
}

impl From<accounts::Model> for UserStat {
    fn from(value: accounts::Model) -> Self {
        Self {
            games_played: value.games_played,
            points: value.games_played,
            cards_had: value.cards_had,
            wins: value.wins,
            loses: value.loses,
            max_points: value.max_points,
        }
    }
}

#[handler]
pub async fn get(Path(id): Path<String>, db: Data<&Arc<DatabaseConnection>>) -> Result<Json<User>, StatusCode> {
    let db = db.deref().as_ref();
    let user = User(queries::accounts::by_uuid_or_login(id.clone())
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?);
    Ok(Json(user))
}

#[handler]
pub async fn get_full(Path(id): Path<String>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let user: UserStat = queries::accounts::by_uuid_or_login(id.clone())
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?.into();
    Ok(Response::builder().body(serde_json::to_string(&user).expect("Failed to serialize UserStat")))
}
