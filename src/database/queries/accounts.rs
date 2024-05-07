
use sea_orm::{prelude::Uuid, Select, ColumnTrait, Condition, TryInsert, EntityTrait, QueryFilter, Set};
use crate::database::entities::{ accounts, prelude::Accounts };

pub fn by_uuid_or_login(id: String, uuid: Uuid) -> Select<Accounts> {
    Accounts::find()
    .filter(
        Condition::any()
            .add(accounts::Column::Login.eq(id))
            .add(accounts::Column::Uuid.eq(uuid))
    )
}

pub fn register(id: String, password: Option<String>) -> TryInsert<accounts::ActiveModel> {
    Accounts::insert(
        accounts::ActiveModel {
            login: Set(id.clone()),
            password: Set(password),
            display_name: Set(id.clone()),
            ..Default::default()
        },
    )
    .on_conflict(sea_orm::sea_query::OnConflict::column(accounts::Column::Id).do_nothing().to_owned())
    .do_nothing()
}