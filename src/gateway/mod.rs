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

fn unwrap_event(event: Result<Payload, Error>) -> Payload {
    match event {
        Ok(payload) => payload,
        Err(e) => Payload::Error(e),
    }
}

#[handler]
pub async fn gateway(
    ws: WebSocket,
    db: Data<&Arc<DatabaseConnection>>,
    players: Data<&Arc<RwLock<sessions::Table>>>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = db.to_owned(); 
    let players = players.to_owned();
    let (sender, mut receiver) = broadcast::channel::<String>(12);
    //let mut receivers = sender.subscribe();
    Ok(
        ws.on_upgrade(move |mut socket| async move {
            let (mut sink, mut stream) = socket.split();
            let hello = Payload::Hello( Hello::new(60) );
            let _ = sink.send(Message::Text(serde_json::to_string(&hello).unwrap_or_default())).await;

            tokio::spawn(async move {
                let mut user_id: Option<Uuid> = None; 
                let db = db.as_ref();
                //let mut rooms = rooms.write().unwrap();
                while let Some(Ok(msg)) = stream.next().await {
                    if let Message::Text(text) = msg {
                        let request = serde_json::from_str(&text);

                        let payload = unwrap_event(
                            if let Ok(request) = request {
                                match request {
                                    Payload::Identify(payload) =>
                                        events::identify(db, payload, &players, sender.clone(), &mut user_id).await,
                                    /*Payload::RoomCreate(payload) => events::room_create(payload, &identity, &rooms, sender.clone()).await,
                                    Payload::RoomUpdate(payload) => events::room_update(payload, &identity, &rooms).await,
                                    Payload::RoomJoin(payload) => 
                                        events::room_join(payload, &identity, &rooms, sender.clone()).await,
                                    Payload::RoomLeave(room_id) => 
                                        events::room_leave(room_id, &identity, &rooms).await,*/
                                    _ => {         
                                        Ok(Payload::Error( Error::Declined ))
                                    },
                                }
                            } else { Err(Error::BadRequest(request.unwrap_err().to_string())) }
                        );
                        let _ = sender.send(payload.to_json_string());
                        //let _ = sink.send(Message::Text(serde_json::to_string(&payload).unwrap_or_default())).await;
                    }
                }
                if let Some(user_id) = user_id {
                    let _ = sleep(Duration::from_secs(60));
                    let mut players = players.write().await;
                    let mut disconnect = false;
                    if let Some(player) = players.get(&user_id) {
                        let player = player.read().await;
                        disconnect = sender.same_channel(&player.sender);
                    }
                    if disconnect { players.remove(&user_id); }
                }
            });

            tokio::spawn(async move {
                while let Ok(text) = receiver.recv().await {
                    if text.contains("./") { continue };
                    let msg = Message::text(text);
                    if let Err(_) = sink.send(msg).await {
                        break;
                    }
                }
            });

        })
    )
}