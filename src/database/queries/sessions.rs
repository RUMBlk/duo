
use sea_orm::{prelude::{ DateTimeWithTimeZone, Uuid }, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Select, SelectGetableTuple, Selector, Set, TryInsert };
use crate::database::entities::{ accounts, sessions, prelude::Sessions };
use chrono::{Days, Local};

pub enum Error {
    DbErr(DbErr),
    NotFound,
    RowExpired
}

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

fn inner_join_account(token: Uuid) -> Select<Sessions> {
    Sessions::find()
    .filter(sessions::Column::Token.eq(token))
    .inner_join(accounts::Entity)
}

pub fn get_account_uuid(token: Uuid) -> Selector<SelectGetableTuple<Uuid>> {
    inner_join_account(token)
        .select_only()
        .column(accounts::Column::Uuid)
        .into_tuple::<Uuid>()
}

pub async fn handle(db: &DatabaseConnection,token: Uuid) -> Result<Uuid, Error> {
    let (id, created_at) = inner_join_account(token)
        .select_only()
        .column(accounts::Column::Uuid)
        .column(accounts::Column::CreatedAt)
        .into_tuple::<(Uuid, DateTimeWithTimeZone)>()
        .one(db)
        .await
        .map_err(|e| Error::DbErr(e))?
        .ok_or(Error::NotFound)?;
    if created_at > Local::now().checked_add_days(Days::new(7)).unwrap_or_default() {
        return Err(Error::RowExpired)
    }
    Ok(id)
}

pub async fn delete(db: &DatabaseConnection, token: Uuid) -> Result<bool, DbErr> {
    let Some(session) = Sessions::find().filter(sessions::Column::Token.eq(token)).one(db).await?
    else { return Ok(false) };
    Sessions::delete(session.clone().into_active_model()).exec(db).await?;
    Ok(true)
}

pub async fn delete_all_of_account(db: &DatabaseConnection, token: Uuid) -> Result<(), DbErr> {
    let account_id = get_account_uuid(token).one(db).await?;
    let sessions = Sessions::find().inner_join(accounts::Entity).filter(accounts::Column::Uuid.eq(account_id)).all(db).await?;
    for session in sessions {
        Sessions::delete(session.into_active_model()).exec(db).await?;
    }
    Ok(())
}