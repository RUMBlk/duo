
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, Select, Set, TryInsert};
use crate::database::entities::{ accounts, prelude::Accounts };

pub fn by_uuid(uuid: Uuid) -> Select<Accounts> {
    Accounts::find()
    .filter(accounts::Column::Uuid.eq(uuid))
}

pub fn by_uuid_or_login(uuid_or_login: String) -> Select<Accounts> {
    let uuid = Uuid::try_parse(&uuid_or_login).unwrap_or_default();
    Accounts::find()
    .filter(
        Condition::any()
            .add(accounts::Column::Login.eq(uuid_or_login))
            .add(accounts::Column::Uuid.eq(uuid))
    )
}

pub fn register(id: String, password: String, display_name: Option<String>) -> TryInsert<accounts::ActiveModel> {
    Accounts::insert(
        accounts::ActiveModel {
            login: Set(id.clone()),
            password: Set(password),
            display_name: Set(display_name.unwrap_or(id.clone())),
            ..Default::default()
        },
    )
    .on_conflict(sea_orm::sea_query::OnConflict::column(accounts::Column::Id).do_nothing().to_owned())
    .do_nothing()
}

pub async fn update<F>(db: &DatabaseConnection, id: Uuid, func: F) -> Result<bool, DbErr>
where F: FnOnce(&accounts::Model, &mut accounts::ActiveModel) {
    let Some(model) = by_uuid(id).one(db).await? else { return Ok(false) };
    let mut active_model = model.clone().into_active_model();
    func(&model, &mut active_model);
    active_model.save(db).await?;
    Ok(true)
}