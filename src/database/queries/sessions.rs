
use sea_orm::{prelude::{ DateTimeWithTimeZone, Uuid }, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Select, SelectGetableTuple, Selector, Set, TryInsert };
use crate::database::entities::{ accounts, sessions, prelude::Sessions };
use chrono::{Days, Local};

pub enum Error {
    DbErr(DbErr),
    NotFound,
    RowExpired
}

pub fn create(id: i64, token: Uuid) -> TryInsert<sessions::ActiveModel> { //Створення нової сесії
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

fn inner_join_account(token: Uuid) -> Select<Sessions> { //SELECT запит з поєднання з таблицею акаунтів
    Sessions::find()
    .filter(sessions::Column::Token.eq(token))
    .inner_join(accounts::Entity) //з'єднання
}

pub fn get_account_uuid(token: Uuid) -> Selector<SelectGetableTuple<Uuid>> { //знайти id власника токену
    inner_join_account(token)
        .select_only() //вибір тільки наступних вказаних стовпців
        .column(accounts::Column::Uuid) //стовпець Uuid
        .into_tuple::<Uuid>() //очікувати значення типу Uuid
}

pub async fn handle(db: &DatabaseConnection,token: Uuid) -> Result<Uuid, Error> { //Обробка використання токенів
    let (id, uuid, created_at) = inner_join_account(token) //з'єднати таблицю sessions з таблицею accounts
        .select_only() //вибрати тільки наступні три стовпця
        .column(sessions::Column::Id)
        .column(accounts::Column::Uuid)
        .column(sessions::Column::CreatedAt)
        .into_tuple::<(i64, Uuid, DateTimeWithTimeZone)>() // очікувати набір значень таких значень
        .one(db) //взяти перший встрічний рядок
        .await
        .map_err(|e| Error::DbErr(e))? //Повернути помилку з'єднання з базою даних
        .ok_or(Error::NotFound)?; //Повернути помилку у випадку не знайденого значення
    if created_at.checked_add_days(Days::new(7)).unwrap_or_default() < Local::now() { 
        return Err(Error::RowExpired) //повернути помилку якщо 7 днів з моменту створення чи поновлення токену пройшло
    } else {
        //інакше взяти рядок sessions за id
        let mut session = Sessions::find_by_id(id).one(db).await
            .expect("Failed to renew token because of DatabaseConnection error")
            .expect("find_by_id panicked trying to find a row by id that existed at the beginning of the function")
            .into_active_model();
        session.created_at = Set(Local::now().into()); //поновити токен
        let _ = session.save(db); //та зберегти
    };
    Ok(uuid)
}

pub async fn delete(db: &DatabaseConnection, token: Uuid) -> Result<bool, DbErr> { //Видалення сесії за токеном
    let Some(session) = Sessions::find().filter(sessions::Column::Token.eq(token)).one(db).await? //знайти рядок з токеном
    else { return Ok(false) }; //якщо немає то повернути false - не змінено
    Sessions::delete(session.clone().into_active_model()).exec(db).await?; //при знайденому рядку - видалити
    Ok(true) //повернути true - змінено
}

pub async fn delete_all_of_account(db: &DatabaseConnection, token: Uuid) -> Result<(), DbErr> { //видалення усіх сесій акаунта
    let account_id = get_account_uuid(token).one(db).await?; //знаходження id акаунта за токеном
    let sessions = Sessions::find().inner_join(accounts::Entity).filter(accounts::Column::Uuid.eq(account_id)).all(db).await?; //знаходження усіх рядків
    for session in sessions {
        Sessions::delete(session.into_active_model()).exec(db).await?; //видалення кожного
    }
    Ok(())
}