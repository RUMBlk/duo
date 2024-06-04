use poem::{
    handler, http::StatusCode, web::{
        Data, Json
    }, Response,
};
use sea_orm::{prelude::Uuid, DatabaseConnection, DbErr, TryInsertResult};
use std::{ops::Deref, sync::Arc};
use crate::database::queries;
use sha256;
use serde::Deserialize;

pub async fn start_session(db: &DatabaseConnection, login_: String, password: String) -> Result<Response, StatusCode> {
    let password = sha256::digest(password.clone()).to_ascii_uppercase();
    let account = queries::accounts::by_uuid_or_login(login_.to_lowercase())
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if account.password == password {
        let token = Uuid::new_v4();
        match queries::sessions::create(account.id, token)
            .exec(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            TryInsertResult::Inserted(_) => Ok(token.to_string()),
            TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
            _ => Err(StatusCode::BAD_REQUEST),
        }
    } else { Err(StatusCode::FORBIDDEN) }
        .map(|token| Response::builder().body(token))
}

#[derive(Debug, Deserialize)]
struct Register {
    login: String,
    password: String,
    display_name: Option<String>,
}
#[handler]
pub async fn register(req: Json<Register>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    if req.password.len() < 6 { return Err(StatusCode::BAD_REQUEST) }
    let password = sha256::digest(req.password.clone()).to_ascii_uppercase();
    match queries::accounts::register(req.login.to_lowercase(), password, req.display_name.clone())
        .exec(db)
        .await
        .map_err(
            |e| 
            match e {
                DbErr::Query(_) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            })?
        {
        TryInsertResult::Inserted(_) => Ok(StatusCode::CREATED),
        TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
        TryInsertResult::Empty => Err(StatusCode::BAD_REQUEST),
    }?;
    start_session(db, req.login.clone(), req.password.clone()).await
}

#[derive(Debug, Deserialize)]
struct Login {
    login: String,
    password: String,
}

#[handler]
pub async fn login(req: Json<Login>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    start_session(db, req.login.clone(), req.password.clone()).await
}

#[derive(Debug, Deserialize)]
struct Exists {
    login: String,
}
#[handler]
pub async fn exists(req: Json<Exists>, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    queries::accounts::by_uuid_or_login(req.login.clone())
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
        .map(|_| StatusCode::OK)
}
