mod database;
mod accounts;
mod rooms;

use poem::{
    get, handler, http::StatusCode, middleware::AddData, patch, post, EndpointExt, Request, Route
};
use shuttle_poem::ShuttlePoem;
use shuttle_runtime::SecretStore;
use std::sync::{ Arc, RwLock };
use std::collections::HashMap;

#[handler]
fn hello_world() -> &'static str {
    "Hello, world!"
}

#[handler]
async fn join_room(token: String, roomcode: String) -> StatusCode {
    StatusCode::OK
}

#[handler]
async fn leave_room(token: String) -> StatusCode {
    StatusCode::OK
}

#[handler]
async fn edit_room(request: &Request) -> StatusCode {
    StatusCode::OK
}

#[shuttle_runtime::main]
async fn poem(#[shuttle_runtime::Secrets] secret_store: SecretStore) -> ShuttlePoem<impl poem::Endpoint> {
    let db = match (
        secret_store.get("DB_HOST"),
        secret_store.get("DB_NAME"),
        secret_store.get("DB_PORT"),
        secret_store.get("DB_USER"),
        secret_store.get("DB_PASS"),
    ) {
        (Some(host), Some(name), Some(port), Some(user), Some(pass)) => {
            let uri = format!(
                "postgres://{}:{}@{}:{}/{}",
                user, pass, host, port, name
            );
            match sea_orm::Database::connect(uri).await {
                Ok(connection) => Ok(connection),
                Err(e) => Err(shuttle_runtime::Error::Database(shuttle_runtime::CustomError::new(e).to_string())),
            }
        },
        _ => Err(shuttle_runtime::Error::Database("Not all database parameters have been provided. The execution is aborted!".to_string())),
    };

    match db {
        Ok(db) => {
            let app = Route::new()
            .at("/api/hello_world/", get(hello_world))
            .at("/api/accounts/:id", get(accounts::login).post(accounts::register))
            .at("/api/rooms/", post(rooms::create_room))
            .at("/api/rooms/:id", patch(edit_room))
            .with(AddData::new(Arc::new(db)))
            .with(AddData::new(Arc::new(RwLock::new(HashMap::<String, rooms::Room>::new()))));
            Ok(app.into())
        }
        Err(e) => {
            Err(e)
        }
    }
}
