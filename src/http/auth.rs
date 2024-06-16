use futures::TryFutureExt;
use poem::{
    handler, http::StatusCode, web::{
        Data, Json
    }, Request, Response
};
use sea_orm::{prelude::Uuid, DatabaseConnection, DbErr, TryInsertResult};
use std::{ops::Deref, sync::Arc};
use crate::database::queries::{self, sessions::{delete, delete_all_of_account }};
use sha256;
use serde::Deserialize;

pub async fn start_session(db: &DatabaseConnection, login_: String, password: String) -> Result<Response, StatusCode> { //Функція для ініціалізації нової сесії
    let password = sha256::digest(password.clone()).to_ascii_uppercase(); //хешування пароля
    let account = queries::accounts::by_uuid_or_login(login_.to_lowercase()) //пошук акаунта за логіном
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? //обробка помилок
        .ok_or(StatusCode::NOT_FOUND)?;

    if account.password == password { //якщо пароль правильний
        let token = Uuid::new_v4(); //створення нового токену
        match queries::sessions::create(account.id, token) //збереження в БД
            .exec(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            TryInsertResult::Inserted(_) => Ok(token.to_string()), //повернення HTTP коду в залежності від результату, Ok - 200
            TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
            _ => Err(StatusCode::BAD_REQUEST),
        }
    } else { Err(StatusCode::FORBIDDEN) } //Інакше повернути помилку
        .map(|token| Response::builder().body(token)) //Повернення відповіді з токеном
}

#[derive(Debug, Deserialize)]
struct Register { //структура, яка задає які поля запит на регістрацію повинен містити
    login: String, //логін
    password: String, //пароль
    display_name: Option<String>, //назва яка буде відображаться
}
#[handler]
pub async fn register(req: Json<Register>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    if req.password.len() < 6 { return Err(StatusCode::BAD_REQUEST) } //повернути помилку якщо пароль менше за 6 символів
    let password = sha256::digest(req.password.clone()).to_ascii_uppercase(); //хешування пароля
    match queries::accounts::register(req.login.to_lowercase(), password, req.display_name.clone()) //виклик функції регістрації та обробка помилок
        .exec(db)
        .await
        .map_err(
            |e| 
            match e {
                DbErr::Query(_) => StatusCode::CONFLICT,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            })?
        {
        TryInsertResult::Inserted(_) => Ok(StatusCode::CREATED),
        TryInsertResult::Conflicted => Err(StatusCode::CONFLICT),
        TryInsertResult::Empty => Err(StatusCode::BAD_REQUEST),
    }?;
    start_session(db, req.login.clone(), req.password.clone()).await //створити нову сесії для нового акаунта
}

#[derive(Debug, Deserialize)]
struct Login { //структура, яка задає які поля запит на авторизацію повинен містити
    login: String, //логін
    password: String, //пароль
}

#[handler]
pub async fn login(req: Json<Login>, db: Data<&Arc<DatabaseConnection>>) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    start_session(db, req.login.clone(), req.password.clone()).await //створити нову сесію
}

#[handler]
pub async fn logout(req: &Request, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let token = Uuid::parse_str(req.header("authorization").ok_or(StatusCode::UNAUTHORIZED)?)
        .map_err(|_| StatusCode::BAD_REQUEST)?; //повернути помилку якщо токен не вказаний в запиті 
    delete(db, token).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR).await?; //виклик функції для видалення сесії за токеном
    Ok(StatusCode::OK)
}

#[handler]
pub async fn logout_all(req: &Request, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let token = Uuid::parse_str(req.header("authorization").ok_or(StatusCode::UNAUTHORIZED)?)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    delete_all_of_account(db, token).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR).await?; //виклик функції для видалення усіх сесії за токеном
    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
struct Exists { //Структура, яка задає які поля запит на перевірку існування акаунта повинен містити
    login: String, //логін
}
#[handler]
pub async fn exists(req: Json<Exists>, db: Data<&Arc<DatabaseConnection>>) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    queries::accounts::by_uuid_or_login(req.login.clone()) //виклик функції пошуку акаунта в БД та перетворення результату у HTTP-код
        .one(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)
        .map(|_| StatusCode::OK)
}
