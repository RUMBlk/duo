
use sea_orm::{prelude::Uuid, ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, Select, Set, TryInsert};
use crate::database::entities::{ accounts, prelude::Accounts };

pub fn by_uuid(uuid: Uuid) -> Select<Accounts> { //Підготувати SELECT запит за id гравця
    Accounts::find()
    .filter(accounts::Column::Uuid.eq(uuid)) //Відфільтрувати всі рядки у який Uuid != наданому id
}

pub fn by_uuid_or_login(uuid_or_login: String) -> Select<Accounts> { //SELECT запит за id або логіном гравця
    let uuid = Uuid::try_parse(&uuid_or_login).unwrap_or_default();
    Accounts::find()
    .filter(
        Condition::any() //OR
            .add(accounts::Column::Login.eq(uuid_or_login))
            .add(accounts::Column::Uuid.eq(uuid))
    )
}

pub fn register(id: String, password: String, display_name: Option<String>) -> TryInsert<accounts::ActiveModel> {
    Accounts::insert(
        accounts::ActiveModel { //Конструювання нової активної моделі для вставлення в базу даних
            login: Set(id.clone()),
            password: Set(password),
            display_name: Set(display_name.unwrap_or(id.clone())),
            ..Default::default()
        },
    )
    .on_conflict(sea_orm::sea_query::OnConflict::column(accounts::Column::Id).do_nothing().to_owned()) //при конфлікті
    .do_nothing() //нічого не робити
}

pub async fn update<F>(db: &DatabaseConnection, id: Uuid, func: F) -> Result<bool, DbErr> //функція для знаходження рядка за id, зміни за допомогою наданої функції F
where F: FnOnce(&accounts::Model, &mut accounts::ActiveModel) { //та збереження
    let Some(model) = by_uuid(id).one(db).await? else { return Ok(false) }; //повернути помилку якщо рядка з наданим id не існує
    let mut active_model = model.clone().into_active_model(); //перетворення в активну модель
    func(&model, &mut active_model); //виклик наданої функції і передача моделей їй
    active_model.save(db).await?; //збереження змін
    Ok(true)
}