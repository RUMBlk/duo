
use serde::Serialize;
use sea_orm::prelude::Uuid;
use tokio::sync::broadcast::Sender;

use crate::{game::rooms::{self , *}, gateway::payloads::RoomPlayerInfo};
use crate::gateway::payloads::{ Payload, RoomPlayer };

#[derive(Debug, Clone, Serialize)]
pub struct Room(pub rooms::Room);

impl Room {
    pub fn create<'a>(name: String, is_public: bool, password: Option<String>, owner: Uuid, max_players: usize, sender: Sender<String>) -> Result<Self, Error<'a>> {
        let mut room = Self(rooms::Room::default());
        room.0.set_name(name)?;
        room.0.is_public = is_public;
        room.0.set_password(password.clone())?;
        room.0.set_max_players(max_players)?;
        room.0.join(password, owner.clone(), sender)?;
        room.0.set_owner(owner.clone())?;
        let thread_room = room.clone();
        thread_room.clone().announce(Payload::RoomCreate(thread_room.0));
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
            self.announce(Payload::RoomUpdate(room.0));
        }
        result
    }

    fn announce(&mut self, payload: Payload ) {
        let room = self.clone();
        tokio::spawn(async move {
            for player in room.clone().0.players() {
                let _ = player.sender.send(payload.to_json_string());
            }
        });
    }
}

impl From<rooms::Room> for Room {
    fn from(value: rooms::Room) -> Self {
        Self { 0: value }
    }
}

impl<'a, 'b> Interaction<'a, 'b> for Room {
    fn set_name(&mut self, name: String) -> Result<(), Error<'b>> {
        self.0.set_name(name)?;
        let room = self.clone();
        self.announce(Payload::RoomUpdate(room.0));
        Ok(())
    }

    fn set_password(&mut self, password: Option<String>) -> Result<(), Error<'b>> {
        self.0.set_password(password)?;
        let room = self.clone();
        self.announce(Payload::RoomUpdate(room.0));
        Ok(())
    }

    fn set_owner(&mut self, owner: Uuid) -> Result<(), Error<'b>> {
        self.0.set_owner(owner)?;
        let room = self.clone();
        self.announce(Payload::RoomUpdate(room.0));
        Ok(())
    }

    fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> {
        self.0.set_max_players(max_players)?;
        let room = self.clone();
        self.announce(Payload::RoomUpdate(room.0));
        Ok(())
    }

    fn join(&mut self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> {
        self.0.join(password, player_id.clone(), sender)?;
        let room = self.clone();
        self.announce(Payload::RoomCreate(room.0.clone()));
        self.announce(Payload::RoomPlayerNew(RoomPlayer::from_room(room.0, player_id)));
        Ok(())
    }

    fn leave(&mut self, player_id: Uuid) -> Result<(), Error<'b>> {
        self.0.leave(player_id)?;
        let room = self.clone();
        self.announce(Payload::RoomPlayerLeft(RoomPlayerInfo::new(room.0.id().to_owned(), player_id)));
        Ok(())
    }

    fn player_switch_ready(&'a mut self, player_id: Uuid) -> Result<(), Error<'b>> {
        self.0.player_switch_ready(player_id)?;
        let room = self.clone();
        self.announce(Payload::RoomPlayerUpdate(RoomPlayer::from_room(room.0, player_id)));
        Ok(())
    }
}