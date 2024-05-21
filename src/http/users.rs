use poem::{
    handler, http::StatusCode, web::{
        Data, Json, Path
    }, Response,
};
use sea_orm::{prelude::Uuid, DatabaseConnection, DbErr, TryInsertResult};
use std::{ops::Deref, sync::Arc};
use crate::database::{self, queries};
use sha256;
use serde::{Deserialize, Serialize, ser::SerializeStruct};

struct User(pub database::entities::accounts::Model);

impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("User", 3)?;
        state.serialize_field("uuid", &self.0.uuid)?;
        state.serialize_field("login", &self.0.login)?;
        state.serialize_field("display_name", &self.0.display_name)?;
        state.end()
    }
}

#[handler]
pub async fn get(Path(id): Path<String>, db: Data<&Arc<DatabaseConnection>>) -> Result<Json<User>, StatusCode> {
    let db = db.deref().as_ref();
    let user = User(queries::accounts::by_uuid_or_login(id.clone())
        .one(db)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?
        .ok_or(StatusCode::NOT_FOUND)?);
    Ok(Json(user))
}
