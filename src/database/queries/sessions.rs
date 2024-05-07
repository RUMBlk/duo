
use sea_orm::{prelude::Uuid, ColumnTrait, Selector, SelectGetableTuple, QuerySelect, TryInsert, EntityTrait, QueryFilter, Set};
use crate::database::entities::{ accounts, sessions, prelude::Sessions };

pub fn create(id: i64, token: Uuid) -> TryInsert<sessions::ActiveModel> {
    Sessions::insert(
        sessions::ActiveModel {
            account: Set(id),
            token: Set(token),
            ..Default::default()
        },
    )
    .on_conflict(sea_orm::sea_query::OnConflict::column(sessions::Column::Id).do_nothing().to_owned())
    .do_nothing()
}

pub fn get_account_uuid(token: Uuid) -> Selector<SelectGetableTuple<Uuid>> {
    Sessions::find()
        .filter(sessions::Column::Token.eq(token))
        .inner_join(accounts::Entity)
        .select_only()
        .column(accounts::Column::Uuid)
        .into_tuple::<Uuid>()
}