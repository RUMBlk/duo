use poem::{
    handler, http::StatusCode, web::{
        Data, Form, Json, Path
    }, Response, Request,
};
use serde::Deserialize;
use sea_orm::{prelude::Uuid, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set, TryInsertResult};
use std::{ops::Deref, sync::Arc};
use super::database::entities;
use entities::prelude as tables;
use sha256;

#[handler]
pub async fn register(Path(id): Path<String>, req: &Request, db: Data<&Arc<DatabaseConnection>>) -> Response {
    let db = db.deref().as_ref();
    let password = match req.header("password") {
        Some(password) => Some(sha256::digest(password).to_ascii_uppercase()),
        None => None,
    };
    let status = match tables::Accounts::insert(
        entities::accounts::ActiveModel {
            login: Set(id.clone()),
            password: Set(password),
            display_name: Set(id.clone()),
            ..Default::default()
        },
    )
    .on_conflict(sea_orm::sea_query::OnConflict::column(entities::accounts::Column::Id).do_nothing().to_owned())
    .do_nothing()
    .exec(db)
    .await {
        Ok(result) => {
            match result {
                TryInsertResult::Inserted(_) => StatusCode::CREATED,
                TryInsertResult::Conflicted => StatusCode::CONFLICT,
                TryInsertResult::Empty => StatusCode::BAD_REQUEST,
            }
        },
        Err(e) => { 
            match e {
                DbErr::Query(_) => StatusCode::CONFLICT,
                _ => StatusCode::BAD_GATEWAY,
            }
        },
    };
    Response::builder()
    .status(status)
    .body("")
}

#[handler]
pub async fn login(Path(id): Path<String>, req: &Request, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let response = match req.header("password") {
        Some(password) => {
            let password = sha256::digest(password).to_ascii_uppercase();
            let uuid = Uuid::try_parse(&id).unwrap_or_default();
            match tables::Accounts::find()
            .filter(
                Condition::any()
                    .add(entities::accounts::Column::Login.eq(id))
                    .add(entities::accounts::Column::Uuid.eq(uuid))
            )
            .one(db)
            .await {
                Ok(account) => {
                    match account {
                        Some(account) => {
                            if account.password == Some(password) {
                                let token = Uuid::new_v4();
                                match tables::Sessions::insert(
                                    entities::sessions::ActiveModel {
                                        account: Set(account.id),
                                        token: Set(token),
                                        ..Default::default()
                                    },
                                )
                                .on_conflict(sea_orm::sea_query::OnConflict::column(entities::sessions::Column::Id).do_nothing().to_owned())
                                .do_nothing()
                                .exec(db)
                                .await {
                                    Ok(result) => {
                                        match result {
                                            TryInsertResult::Inserted(_) => Ok(token.to_string()),
                                            TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
                                            TryInsertResult::Empty => Err(StatusCode::BAD_REQUEST),
                                        }
                                    },
                                    Err(_) => Err(StatusCode::BAD_GATEWAY),
                                }
                            } else {
                                Err(StatusCode::FORBIDDEN)
                            }
                        },
                        None => Err(StatusCode::NOT_FOUND),
                    }
                },
                Err(_) => Err(StatusCode::BAD_GATEWAY),
            }
        },
        None => Err(StatusCode::BAD_REQUEST),
    };

    let status = match response {
        Ok(_) => StatusCode::OK,
        Err(e) => e, 
    };

    Ok(
        Response::builder()
        .status(status)
        .body(response.unwrap_or_default())
    )
}