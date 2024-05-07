use poem::{
    handler, http::StatusCode, web::{
        Data, Path, Json
    }, Response,
};
use sea_orm::{prelude::Uuid, DatabaseConnection, DbErr, TryInsertResult};
use std::{ops::Deref, sync::Arc};
use crate::database::queries;
use sha256;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Register {
    password: Option<String>,
}
#[handler]
pub async fn register(Path(id): Path<String>, req: Json<Register>, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let password = req.password.clone().and_then(|password| Some(sha256::digest(password).to_ascii_uppercase()));
    match queries::accounts::register(id, password)
    .exec(db)
    .await
    .map_err(|e| match e {
        DbErr::Query(_) => StatusCode::CONFLICT,
        _ => StatusCode::BAD_GATEWAY,
    })? {
        TryInsertResult::Inserted(_) => Ok(StatusCode::CREATED),
        TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
        TryInsertResult::Empty => Err(StatusCode::BAD_REQUEST),
    }
}

#[derive(Debug, Deserialize)]
struct Login {
    password: String,
}
#[handler]
pub async fn login(Path(id): Path<String>, req: Json<Login>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let password = sha256::digest(req.password.clone()).to_ascii_uppercase();
    let uuid = Uuid::try_parse(&id).unwrap_or_default();
    let account = queries::accounts::by_uuid_or_login(id, uuid)
    .one(db)
    .await
    .map_err(|_| StatusCode::BAD_GATEWAY)?
    .ok_or(StatusCode::NOT_FOUND)?;

    if account.password == Some(password) {
        let token = Uuid::new_v4();
        match queries::sessions::create(account.id, token)
        .exec(db)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)? {
            TryInsertResult::Inserted(_) => Ok(token.to_string()),
            TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
            _ => Err(StatusCode::BAD_REQUEST),
        }
    } else { Err(StatusCode::FORBIDDEN) }
        .map(|token| Response::builder().body(token))
}

#[handler]
pub async fn exist_checker(Path(id): Path<String>, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let uuid = Uuid::try_parse(&id).unwrap_or_default();
    queries::accounts::by_uuid_or_login(id, uuid)
    .one(db)
    .await
    .map_err(|_| StatusCode::BAD_GATEWAY)?
    .ok_or(StatusCode::NOT_FOUND)
    .map(|_| StatusCode::OK)
}
