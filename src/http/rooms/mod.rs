pub mod game;

use poem::{handler, http::StatusCode, web::{ self, Data, Json, Path }, Request, Response };
use sea_orm::{ prelude::Uuid, DatabaseConnection };
use serde::{Deserialize, Serialize};
use tokio::sync::{ RwLock, RwLockWriteGuard};
use std::{ ops::Deref, sync::Arc };
use crate::{ 
    Rooms,
    game::rooms::{self, Room, Partial},
    database::queries,
    gateway::sessions::User,
    runtime_storage::Table,
};

async fn prelude<'a>( //функція для валідації токена, пошук акаунта за токеном та бронювання показників 
    db: &'a DatabaseConnection,
    auth: Option<&'a str>,
    players_ptr: &'a Arc<RwLock<crate::Players>>,
    rooms_ptr: &'a Arc<RwLock<Rooms>>
) -> Result<(RwLockWriteGuard<'a, crate::Players>, RwLockWriteGuard<'a, crate::Rooms>, User), StatusCode> {
    let auth = Uuid::parse_str(
        auth
        .ok_or(StatusCode::BAD_REQUEST)?
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let player_id = queries::sessions::handle(db, auth).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let players = players_ptr.write().await;
    let player = players.get(&player_id).ok_or(StatusCode::FORBIDDEN)?.clone();

    let rooms = rooms_ptr.write().await;
    Ok((players, rooms, player))
}

fn limit() -> usize { 100 } //функція для задання значення за замовченням при десеріалізації

#[derive(Deserialize)]
struct RoomQuery { //структура, яка описує необхідні Query параметри у шляху, необхідна для пажировки списку кімнат
    #[serde(default)] //використання значення usize за замовченням - 0
    after: usize, //поле, яке указує після якої кількості кімнат необхідно почати формувати список для відповіді
    #[serde(default = "limit")] //виклик функції, якщо значення не отримано
    limit: usize, //поле, яке указує максимальну кількість кімнат які список для відповіді має містити
}

#[handler]
pub async fn get_rooms_list(query: web::Query<RoomQuery>, rooms: Data<&Arc<RwLock<Rooms>>>) -> Json<Vec<Partial>> { //функція для формування списку
    let rooms = rooms.read().await; //замок на таблицю кімнат
    let mut rooms_vec: Vec<Partial> = rooms //таблиця кімнат
        .iter() //її метод для ітерації
        .filter(|room| room.is_public) //фільтр значеня за полем is_public
        .skip(query.after) //пропуск перших query.after значень в rooms
        .take(query.limit) //беремо query.limit значень
        .map(|room| Partial(room.clone().into())) //перетворюємо в часткову модель
        .collect(); //та збираємо у новий вектор
    Json(rooms_vec) //надсилання відповіді
}

#[derive(Deserialize)]
struct RoomCreate { //структура, яка задає необхідні поля, які запит повинен містити
    name: String, //назва кімнати
    is_public: bool, //публічність
    password: Option<String>, //пароль, поле не обов'язкове
    max_players: usize, //максимальна кількість гравців
}

#[handler]
pub async fn create(
    req: &Request,
    body: Json<RoomCreate>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?; //виклик функції валідації токена та бронювання показників
    if let Some(_) = &player.room { //якщо гравець вже в кімнаті то повернути помилку
        return Err(StatusCode::FORBIDDEN);
    }

    let mut room = Room::create(body.name.clone(), body.is_public, body.password.clone(), *player.uuid(), body.max_players, player.sender.clone()).await
        .map_err(|_| StatusCode::BAD_REQUEST)?; //створити кімнату та обробити помилки
    while let Some(_) = rooms.get(&room.clone()) { //доки в таблиці вже існує кімната з таким ідентифікатором
        room.regenerate_id() //згенерувати новий
    };
    rooms.insert(room.clone()); //вставити кімнату в таблицю
    player.room = Some(room.id().clone()); //задання кімнати для відновлення сесії
    players.replace(player); //оновлення користувача в таблиці користувачів
    Ok(Response::builder() //відповідь
        .body(serde_json::to_string(&room).expect("Failed to serialize RoomResult")))

}

#[derive(Deserialize)]
struct RoomUpdate { //структура, яка описує необхідні поля, які запит повинен мати для оновлення параметрів кімнати
    name: Option<String>, //повторює RoomCreate, але з відмінністю в обов'язковості полів
    is_public: Option<bool>, //усі поля не є обов'язковими, але оновлюються тільки ті, що надані
    password: Option<String>,
    owner: Option<Uuid>,
    max_players: Option<usize>,
}


#[handler]
pub async fn update( //функція для оновлення параметрів кімнати
    Path(id): Path<String>,
    req: &Request,
    body: Json<RoomUpdate>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Response, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let mut room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?; //повернути помилку якщо кімнати з наданим id не існує

    if *room.owner() != *player.uuid() { return Err(StatusCode::FORBIDDEN) } //якщо клієнт не є власником кімнати, повернути помилку

    let mut room = room.clone();
    let mut result = Vec::new(); //вектор результатів
    if let Some(ref value) = body.name { result.push(room.set_name(value.to_string())) } //перевірка на наявність поля у запиті та спроба оновити значення
    if let Some(value) = body.is_public { room.is_public = value; result.push(Ok(())) } //повернення результату у вектор результатів
    if let Some(ref value) = body.password { result.push(room.set_password(Some(value.to_string()))) }
    if let Some(value) = body.owner { result.push(room.set_owner(value)) }
    if let Some(value) = body.max_players { result.push(room.set_max_players(value)) }
    for i in &result { 
            if let Err(_) = i { return Ok( //якщо вектор має хоч одну помилку, повернути відповідь з усима помилками не змінюючи кімнату
                Response::builder().status(StatusCode::BAD_REQUEST).body(
                    serde_json::to_string(&result).unwrap()
                )
            )
        }
    };
    rooms.replace(room); //інакше оновити кімнату
    Ok(Response::builder().status(StatusCode::OK).body("")) //та повернути відповідь з статусом 200 успіх

}

#[derive(Serialize, Deserialize)]
struct RoomJoin { //Структура, яка описує поля, запит на необхідні для підключення до кімнати повинен мати
    password: Option<String>,
}

#[handler]
pub async fn join( //функція для приєднання до кімнати
    Path(id): Path<String>,
    req: &Request,
    body: Json<RoomJoin>,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<Json<Room>, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone(); //пошук кімнати в таблиці
    if let Some(room_id) = &player.room { //сесія гравця вже містить якусь кімнату
        if id != *room_id { //і якщо ця кімната не є кімнатою до якої гравець хоче доєднатися
            return Err(StatusCode::FORBIDDEN); //повернути помилку
        } else {
            return Ok(Json(room)) //інакше надати інформацію про кімнату
        }
    }
    room.join(body.password.clone(), *player.uuid(), player.sender.clone()).await.map_err(|_| StatusCode::FORBIDDEN)?; //якщо гравець не доєднаний до кімнати, доєднатися
    player.room = Some(room.id().clone()); //оновити значення кімнати в сесії
    players.replace(player); //оновити сесію в таблиці
    Ok(Json(room))  //відповідь
}


#[handler]
pub async fn leave( //функція для виходу з кімнати
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (mut players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let mut room = rooms.get::<String>(&id).ok_or(StatusCode::NOT_FOUND)?.clone(); //пошук кімнати
    let leave = room.leave(player.uuid().clone()).await; //виклик методу виходу кімнати, збереження результату
    if let Err(rooms::Error::CantAssignNewOwner) = leave { //якщо не залишилося гравців
        rooms.remove(&room.clone()); //то видалити кімнату з таблиці
    } else if let Ok(true) = leave { //інакше якщо гравці є і власник змінився
        rooms.replace(room); //то оновити кімнату в таблиці
    } else {
        leave.map_err(|_| StatusCode::FORBIDDEN)?; //інакше видати помилку
    }
    player.room = None; //очистити поле кімнати з сесії
    players.replace(player); //оновити сесію в таблиці
    Ok(StatusCode::OK) //успішна відповідь
}

#[handler]
pub async fn ready( //функція для перемикання готовності гравця
    Path(id): Path<String>,
    req: &Request,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<Rooms>>>,
) -> Result<StatusCode, StatusCode> {
    let db = db.deref().as_ref();
    let (_players, mut rooms, mut player) =
        prelude(db, req.header("authorization"), players_ptr.deref(), rooms_ptr.deref()).await?;
    let room = rooms.get(&id).ok_or(StatusCode::NOT_FOUND)?; //пошу кімнати
    room.player_switch_ready(player.uuid().clone()).await.map_err(|_| StatusCode::FORBIDDEN)?; //виклик методу перемикання та обробка помилки
    Ok(StatusCode::OK) //відповідь
}