mod database;
mod gateway;
mod http;
mod game;

use game::rooms;
use poem::{
    get, handler, head, middleware::{ AddData, Cors }, patch, post, EndpointExt, Route
};
use shuttle_poem::ShuttlePoem;
use shuttle_runtime::SecretStore;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

use http::*;

pub type Players = HashSet::<gateway::sessions::User>;
pub type Rooms = HashSet::<rooms::Room>;

#[handler]
fn hello_world() -> &'static str {
    "Hello, world!"
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
            .at("/api/hello_world", get(hello_world))
            .at("/api/gateway", get(gateway::gateway))
            .at("/api/auth/register", head(auth::exists).post(auth::register))
            .at("/api/auth/login", post(auth::login))
            .at("/api/users/:id", get(users::get))
            .at("/api/rooms", get(http::rooms::get_rooms_list).post(http::rooms::create))
            .at("/api/rooms/:id", patch(http::rooms::update))
            .at("/api/rooms/:id/join", post(http::rooms::join))
            .at("/api/rooms/:id/ready", post(http::rooms::ready))
            .at("/api/rooms/:id/leave", post(http::rooms::leave))
            .with(Cors::new().allow_origin_regex("*"))
            .with(AddData::new(Arc::new(db)))
            .with(AddData::new(Arc::new(RwLock::new(Players::new()))))
            .with(AddData::new(Arc::new(RwLock::new(Rooms::new()))));
            Ok(app.into())
        }
        Err(e) => {
            Err(e)
        }
    }
}
