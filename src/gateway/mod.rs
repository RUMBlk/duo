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
use crate::game::rooms;
use crate::runtime_storage::Table;

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
    players_ptr: Data<&Arc<RwLock<crate::Players>>>,
    rooms_ptr: Data<&Arc<RwLock<crate::Rooms>>>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = db.to_owned(); 
    let players = players_ptr.to_owned();
    let rooms = rooms_ptr.to_owned();
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
                                        events::identify(db, payload, &players, &rooms.clone(), sender.clone(), &mut user_id).await,
                                    _ => {         
                                        Ok(Payload::Error( Error::Declined ))
                                    },
                                }
                            } else { Err(Error::BadRequest(request.unwrap_err().to_string())) }
                        );
                        let _ = sender.send(payload.to_json_string());
                    }
                }
                if let Some(user_id) = user_id {
                    let _ = sleep(Duration::from_secs(60)).await;
                    let mut players = players.write().await;
                    let disconnect = if let Some(player) = players.get(&user_id) {
                        if sender.same_channel(&player.sender) { Some(player.clone()) } else { None }
                    } else { None };
                    if let Some(player) = disconnect { 
                        let mut rooms = rooms.write().await;
                        if let Some(mut room) = player.room.and_then(|room_id| rooms.get(&room_id).cloned()) {
                            match room.leave(user_id).await {
                                Err(rooms::Error::CantAssignNewOwner) => { rooms.remove(&room); },
                                Ok(_) | Err(_) => { rooms.replace(room); },
                            }
                        }
                        players.remove(&user_id);
                    }
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