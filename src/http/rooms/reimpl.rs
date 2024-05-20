use std::hash::Hash;

use serde::{ ser::SerializeStruct, Serialize };
use sea_orm::prelude::Uuid;

use crate::{game::rooms::*, gateway::payloads::RoomPlayerInfo};
use crate::gateway::sessions::User;
use crate::gateway::payloads::{ Payload, RoomPlayer };
use tokio::sync::broadcast::Sender;

use super::player;

#[derive(Debug, Clone)]
pub struct Room(pub crate::Room);

impl Room {
    pub fn create<'a>(name: String, is_public: bool, password: Option<String>, owner: User, max_players: usize) -> Result<Self, Error<'a>> {
        let mut room = Self(crate::Room::default());
        room.0.set_name(name)?;
        room.0.is_public = is_public;
        room.0.set_password(password.clone())?;
        room.0.set_max_players(max_players)?;
        room.0.join(password, owner.clone().into())?;
        room.0.set_owner(owner.uuid().clone())?;
        let thread_room = room.clone();
        tokio::spawn(async move {
            thread_room.clone().announce(Payload::RoomCreate(thread_room)).await;
        });
        Ok(room)
    }

    pub fn update<'a, 'b>(&'a mut self, name: Option<String>, is_public: Option<bool>, password: Option<String>, owner: Option<Uuid>, max_players: Option<usize>) -> Vec<Result<(), Error<'b>>>{
        let mut result = Vec::new();
        if let Some(value) = name { result.push(self.0.set_name(value)) }
        if let Some(value) = is_public { self.0.is_public = value; result.push(Ok(())) }
        if let Some(value) = password { result.push(self.0.set_password(Some(value))) }
        if let Some(value) = owner { result.push(self.0.set_owner(value)) }
        if let Some(value) = max_players { result.push(self.0.set_max_players(value)) }
        if result.len() > 0 { 
            let room = self.clone();
            tokio::spawn(async move {
                room.clone().announce(Payload::RoomUpdate(room)).await
            });
        }
        result
    }

    async fn announce(&mut self, payload: Payload ) {
        for player in self.0.players().clone() {
            if let Err(_) = player.data.sender().send(payload.to_json_string()) {
                self.leave(player.data);
            }
        }
    }
}

impl From<crate::Room> for Room {
    fn from(value: crate::Room) -> Self {
        Self { 0: value }
    }
}

impl<'a, 'b> Interaction<'a, 'b, player::Data, Uuid> for Room {
    fn set_name(&mut self, name: String) -> Result<(), Error<'b>> {
        self.0.set_name(name)?;
        let room = self.clone();
        tokio::spawn(async move {
            room.clone().announce(Payload::RoomUpdate(room)).await;
        });
        Ok(())
    }

    fn set_password(&mut self, password: Option<String>) -> Result<(), Error<'b>> {
        self.0.set_password(password)?;
        let room = self.clone();
        tokio::spawn(async move {
            room.clone().announce(Payload::RoomUpdate(room)).await;
        });
        Ok(())
    }

    fn set_owner(&mut self, ownership: Uuid) -> Result<(), Error<'b>> {
        self.0.set_owner(ownership)?;
        let room = self.clone();
        tokio::spawn(async move {
            room.clone().announce(Payload::RoomUpdate(room)).await;
        });
        Ok(())
    }

    fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> {
        self.0.set_max_players(max_players)?;
        let room = self.clone();
        tokio::spawn(async move {
            room.clone().announce(Payload::RoomUpdate(room)).await;
        });
        Ok(())
    }

    fn join(&mut self, password: Option<String>, player: player::Data) -> Result<(), Error<'b>> {
        self.0.join(password, player.clone())?;
        let room = self.clone();
        tokio::spawn(async move {
            let _ = player.sender().send(Payload::RoomCreate(room.clone()).to_json_string());
            room.clone().announce(Payload::RoomPlayerNew(RoomPlayer::from_room(room.0, player))).await;
        });
        Ok(())
    }

    fn leave(&mut self, player: player::Data) -> Result<(), Error<'b>> {
        if *self.0.owner() == Some(*player.id()) {
            return Err(Error::Forbidden("Owners can't leave their room, consider to transfer ownership or delete the room"))
        }
        self.0.players_mut().remove(&player::Data::from(player.clone()));
        let room = self.clone();
        tokio::spawn(async move {
            room.clone().announce(Payload::RoomPlayerLeft(RoomPlayerInfo::new(room.0.id().to_owned(), player.id().to_owned()))).await;
        });
        Ok(())
    }
}

impl Serialize for Room {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut state = serializer.serialize_struct("room", 6)?;
        state.serialize_field("id", self.0.id())?;
        state.serialize_field("name", self.0.name())?;
        state.serialize_field("is_public", &self.0.is_public)?;
        state.serialize_field("password", self.0.password())?;
        state.serialize_field("owner", self.0.owner())?;
        state.serialize_field("max_players", self.0.max_players())?;
        state.serialize_field("players", &self.0.players())?;
        state.end()
    }
}

pub struct RoomPartial(pub crate::Room);

impl Serialize for RoomPartial {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        let mut state = serializer.serialize_struct("room", 6)?;
        state.serialize_field("id", self.0.id())?;
        state.serialize_field("name", self.0.name())?;
        state.serialize_field("is_public", &self.0.is_public)?;
        state.serialize_field("password", &self.0.password().is_some() )?;
        state.serialize_field("owner", self.0.owner())?;
        state.serialize_field("max_players", self.0.max_players())?;
        state.serialize_field("players", &self.0.players().len())?;
        state.end()
    }
}
