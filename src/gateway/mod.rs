pub mod payloads;
pub mod events;

use futures_util::{ SinkExt, stream::FuturesUnordered };
use poem::{
    handler, http::{request, response, StatusCode}, web::{ self, websocket::{Message, WebSocket }, Data, Json, Path }, IntoResponse, Request, Response
};
use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::{prelude::Uuid, DatabaseConnection, Iden};
use std::{collections::HashMap, hash::Hash, ops::Deref, sync::Arc, time::Duration };
use crate::{auth, database::queries, game::room::{self, Room}};
use tokio::{stream, sync::{ broadcast::{ self, Receiver}, RwLock }, time::sleep};
use futures_util::StreamExt;
use std::collections::HashSet;

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
    rooms: Data<&Arc<RwLock<HashMap<String, Room>>>>,
) -> Result<impl IntoResponse, StatusCode> {
    let db = db.to_owned(); 
    let rooms = rooms.to_owned();
    let (sender, mut receiver) = broadcast::channel::<String>(12);
    //let mut receivers = sender.subscribe();
    let mut identity = None;

    Ok(
        ws.on_upgrade(move |mut socket| async move {
            let (mut sink, mut stream) = socket.split();
            let hello = Payload::Hello( Hello::new(60) );
            let _ = sink.send(Message::Text(serde_json::to_string(&hello).unwrap_or_default())).await;

            tokio::spawn(async move {
                let db = db.as_ref();
                //let mut rooms = rooms.write().unwrap();
                while let Some(Ok(msg)) = stream.next().await {
                    if let Message::Text(text) = msg {
                        let request = serde_json::from_str(&text);

                        let payload = unwrap_event(
                            if let Ok(request) = request {
                                match request {
                                    Payload::Identify(payload) =>
                                        events::identify(db, payload, &mut identity).await,
                                    Payload::RoomCreate(_, payload) => events::room_create(payload, &identity, &rooms, sender.clone()).await,
                                    Payload::RoomUpdate(room_id, payload) => events::room_update(room_id, payload, &identity, &rooms).await,
                                    Payload::RoomJoin(payload) => 
                                        events::room_join(payload, &identity, &rooms, sender.clone()).await,
                                    Payload::RoomLeave(room_id) => 
                                        events::room_leave(room_id, &identity, &rooms).await,
                                    _ => {         
                                        Ok(Payload::Hello( Hello::new(60) ))
                                    },
                                }
                            } else { Err(Error::BadRequest) }
                        );
                        let _ = sender.send(payload.to_json_string());
                        //let _ = sink.send(Message::Text(serde_json::to_string(&payload).unwrap_or_default())).await;
                    }
                }
            });

            tokio::spawn(async move {
                while let Ok(text) = receiver.recv().await {
                    let msg = Message::text(text);
                    let _ = sink.send(msg).await;
                }
            });

        })
    )
}