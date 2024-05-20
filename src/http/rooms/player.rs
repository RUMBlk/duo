use std::hash::Hash;

use serde::{ ser::SerializeStruct, Serialize };
use sea_orm::prelude::Uuid;

use crate::{game::rooms::*, gateway::payloads::RoomPlayerInfo};
use crate::gateway::sessions::User;
use crate::gateway::payloads::{ Payload, RoomPlayer };
use tokio::sync::broadcast::Sender;

#[derive(Debug, Clone)]
pub struct Data {
    id: Uuid,
    sender: Sender<String>,
}

impl Data {
    pub fn new(id: Uuid, sender: Sender<String>) -> Self {
        Self { id, sender }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn sender(&self) -> &Sender<String> {
        &self.sender
    }
}

impl From<User> for Data {
    fn from(value: User) -> Self {
        Self::new(*value.uuid(), value.sender)
    }
}

impl Eq for Data { }
impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Data {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Serialize for Data {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let state = serializer.serialize_struct("id", 1)?;
        state.end()
    }
}