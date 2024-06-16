pub mod payloads;
pub mod events;
pub mod sessions;

use futures_util::SinkExt;
use poem::{
    handler, http::StatusCode, web::{ websocket::{Message, WebSocket }, Data }, IntoResponse
};
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection};
use std::{sync::Arc, time::Duration};
use tokio::sync::{ broadcast, RwLock };
use tokio::time::sleep;
use futures_util::StreamExt;
use payloads::*;
use crate::{game::rooms, runtime_storage::Table};

fn unwrap_event(event: Result<Payload, Error>) -> Payload { //Розгортач результатів подій
    match event {
        Ok(payload) => payload,
        Err(e) => Payload::Error(e),
    }
}

#[handler]
pub async fn gateway(
    ws: WebSocket,
    db: Data<&Arc<DatabaseConnection>>,
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<crate::Rooms>>>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = db.to_owned(); //Доставання значення з показника
    let players = players_ptr.to_owned(); //Доставання замка таблиці гравців з показника
    let rooms = rooms_ptr.to_owned(); //Доставання замка таблиці кімнат з показника
    let (sender, mut receiver) = broadcast::channel::<String>(12); //створення нового каналу
    //let mut receivers = sender.subscribe();
    Ok(
        ws.on_upgrade(move |mut socket| async move { //при перетворенні http з'єднання в gateway
            let (mut sink, mut stream) = socket.split(); //розділити з'єднання на вхідний та вихідний потік
            let hello = Payload::Hello( Hello::new(60) ); //Привітальне повідомлення
            let _ = sink.send(Message::Text(serde_json::to_string(&hello).unwrap_or_default())).await; //Надсилання прив. повідомлення

            //Створення нового потоку для вхідних значень
            tokio::spawn(async move {
                let mut user_id: Option<Uuid> = None; //id користувача у цьому замиканні 
                let db = db.as_ref(); //Показник на з'єднання БД
                //let mut rooms = rooms.write().unwrap();
                while let Some(Ok(msg)) = stream.next().await { //Доки надходять повідомлення
                    if let Message::Text(text) = msg { //Якщо повідомлення містить текст
                        let request = serde_json::from_str(&text); //Десеріалізувати з JSON

                        let payload = unwrap_event( //Розгорнути результат наступної події
                            if let Ok(request) = request { //якщо десеріалізація пройшла успішно
                                match request {
                                    Payload::Identify(payload) => //ідентифікація та авторизація акаунта за наданим токеном
                                        events::identify(db, payload, &players, &rooms.clone(), sender.clone(), &mut user_id).await,
                                    _ => {         // всі інші повідомлення - відмова
                                        Ok(Payload::Error( Error::Declined )) 
                                    },
                                }
                            } else { Err(Error::BadRequest(request.unwrap_err().to_string())) } //якщо десеріалізація повернула помилку, повернути її
                        );
                        let _ = sender.send(payload.to_json_string()); //Надіслати payload клієнту
                    }
                }
                //якщо з'єднання закрилося
                if let Some(user_id) = user_id { //якщо користувач авторизувався
                    let _ = sleep(Duration::from_secs(60)).await; //зачекати 60 секунд
                    let mut players = players.write().await; //замок на таблицю гравців
                    let disconnect = if let Some(player) = players.get(&user_id) { 
                        if sender.same_channel(&player.sender) { Some(player.clone()) } else { None } 
                        //disconnect = якщо сесія все ще має той самий надсилач, тобто вона не було відновлена
                    } else { None };
                    if let Some(player) = disconnect { //якщо disconnect 
                        let mut rooms = rooms.write().await; //замок на таблицю кімнат
                        if let Some(mut room) = player.room.and_then(|room_id| rooms.get(&room_id).cloned()) { //дістати кімнату
                            match room.leave(user_id).await { // викликати метод виходу гравця з кімнати
                                Err(rooms::Error::CantAssignNewOwner) => { rooms.remove(&room); }, //видалити кімнату якщо вона пуста
                                Ok(_) | Err(_) => { rooms.replace(room); }, //інашке оновити кімнату
                            }
                        }
                        players.remove(&user_id); //видалити сесію
                    }
                }
            });

            //створити новий поток надсилання значень
            tokio::spawn(async move {
                //доки канал відкритий
                while let Ok(text) = receiver.recv().await {
                    if text.contains("./") { continue };
                    let msg = Message::text(text);
                    if let Err(_) = sink.send(msg).await { //надіслати вхідний текст клієнту
                        break;
                    }
                }
            });

        })
    )
}