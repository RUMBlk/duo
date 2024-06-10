pub mod player;

use std::{borrow::Borrow, hash::Hash, ops::Deref, sync::Arc};
use sea_orm::prelude::Uuid;
use tokio::sync::{ RwLock, broadcast::Sender };
use random_string;
use serde::{ser::SerializeStruct, Serialize};
use player::Player;
use crate::{
    gateway::{ events::TableEvents, payloads::Payload },
    runtime_storage::{ DataTable, SharedTable },
    game::gameplay::Ok,
};
use futures::executor;
use super::gameplay::{self, Game};

#[derive(Debug, Serialize)]
pub enum Error<'a> {
    PlayerNotInRoom,
    BadArgument(&'a str),
    Forbidden(&'a str),
    CantAssignNewOwner,
    NoGame,
    GameAlreadyStarted,
    Full,
    Game(gameplay::Error)
}

#[derive(Debug, Clone)]
pub struct Room {
    id: String,
    name: String,
    pub is_public: bool,
    password: Option<String>,
    owner: Uuid,
    max_players: usize,
    players: Arc<RwLock<DataTable<Player>>>,
    pub game: Option<Arc<RwLock<Game>>>,
}

impl Default for Room
 {
    fn default() -> Self {
        Self {
            id: Self::generate_id(),
            name: String::from("Room"),
            is_public: false,
            password: None,
            owner: Uuid::default(),
            max_players: 2,
            players: Arc::new(RwLock::new(DataTable::new())),
            game: None,
        }
    }
}

impl<'a, 'b> Room
{
    pub async fn create(name: String, is_public: bool, password: Option<String>, owner: Uuid, max_players: usize, sender: Sender<String>) -> Result<Self, Error<'b>> {
        let mut room = Self::default();
        room.set_name(name)?;
        room.is_public = is_public;
        room.set_password(password.clone())?;
        room.set_max_players(max_players)?;
        room.join(password, owner.clone(), sender).await?;
        room.set_owner(owner.clone())?;
        Ok(room)
    }

    pub fn announce(&self, content: String ) {
        let room = self.clone();
        tokio::spawn(async move {
            for player in &**room.players().read().await {
                let _ = player.sender.send(content.clone());
            }
        });
    }

    pub fn generate_id() -> String {
        random_string::generate(6, "0123456789")
    }

    pub fn regenerate_id(&mut self) {
        self.id = Self::generate_id()
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }

    pub fn owner(&self) -> &Uuid {
        &self.owner
    }

    pub fn max_players(&self) -> &usize {
        &self.max_players
    }

    pub fn players(&self) -> &Arc<RwLock<DataTable<Player>>> {
        &self.players
    }

    pub fn game(&self) -> &Option<Arc<RwLock<Game>>> {
        &self.game
    }

    pub fn set_name(&mut self, name: String) -> Result<(), Error<'b>> {
        if !name.is_empty() { self.name = name; }
        else { return Err(Error::BadArgument("name can't be an empty string")) }
        Ok(())
    }

    pub fn set_password(&mut self, password: Option<String>) -> Result<(), Error<'b>> {
        if let Some(ref pass) = password {
            if pass.len() < 32 { 
                self.password = if pass.is_empty() { None } else { password }
            }
            else { return Err(Error::BadArgument("password can't be longer than 32 characters")) }
        }
        Ok(())
    }

    pub fn set_owner(&mut self, owner: Uuid) -> Result<(), Error<'b>> {
        self.owner = owner;
        Ok(())
    }

    pub fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> {
        if max_players < 2 { return Err( Error::BadArgument("max_players can't be lower than 2") ) }
        else { self.max_players = max_players }
        Ok(())
    }

    pub async fn join(&'a self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> {
        let mut players = self.players.write().await;
        if let Some(pass) = &self.password {
            if Some(pass) != password.as_ref() {
                return Err(Error::Forbidden("Wrong password"));
            }
        }
        if players.len() >= self.max_players && !players.contains(&player_id) {
            return Err(Error::Full)
        }
        

        //if self.players.contains(&player) { return Err(Error::PlayerAlreadyInRoom) }
        players.shared_insert(player::Player::new(player_id, sender));
        Ok(())
    }

    pub async fn leave(&'a mut self, player_id: Uuid) -> Result<bool, Error<'b>> {
        let mut players = self.players.write().await;
        if !players.shared_remove(&player_id) {
            return Err(Error::PlayerNotInRoom);
        };
        let changed = self.owner == player_id;
        if changed {
            self.owner = players.iter().next().ok_or(Error::CantAssignNewOwner)?.id;
        }
        Ok(changed)
    }

    pub async fn player_switch_ready(&'a self, player_id: Uuid) -> Result<(), Error<'b>> {
        let mut players = self.players.write().await;
        players.shared_update(&player_id, |player| {
            player.is_ready = !player.is_ready;
            Ok::<(), ()>(())
        }).unwrap_or(None).ok_or(Error::PlayerNotInRoom)?;
        Ok(())
    }

    pub async fn player_update_sender(&'a self, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> {
        let mut players = self.players.write().await;
        players.shared_update(&player_id, |player| {
            player.sender = sender.clone();
            Ok::<(), ()>(())
        }).unwrap_or(None).ok_or(Error::PlayerNotInRoom)?;
        drop(players);
        let _ = sender.send(Payload::RoomCreate(self.clone()).to_json_string());
        if let Some(game) = &self.game {
            let mut game = game.write().await;
            game.player_update_sender(player_id, sender.clone());
        }
        Ok(())
    }

    pub async fn start_game(&'a mut self) -> Result<(), Error<'b>> {
        match self.game {
            Some(_) => Err(Error::GameAlreadyStarted),
            None => { 
                let game_obj = Game::new(self.players.read().await.deref().deref().clone())
                    .map_err(|e| Error::Game(e))?;
                self.game = Some(Arc::new(RwLock::new(game_obj.clone())));
                let game = self.game.as_ref().unwrap().read().await;
                game.announce(Payload::GameStarted(game_obj).to_json_string());
                Ok(())
            }
        }
    }

    pub async fn play_game(&'a self, player_id: Uuid, card_id: Option<usize>) -> Result<Ok, Error<'b>> {
        match &self.game {
            Some(game) => {
                let mut game =game.write().await;
                let result = game.play(player_id, card_id).map_err(|e| Error::Game(e))?;
                match result {
                    Ok::GameOver(ref players ) => {
                        let mut room_players = self.players.write().await;
                        for loser in players.iter() {
                            let _ = room_players.shared_update(loser.id(), |player| {
                                player.points += loser.points();
                                Ok::<(), ()>(())
                            });
                        };
                        self.announce(Payload::GameOver(players.clone()).to_json_string());
                    },
                    _ => {},
                };
                Ok(result)
            },
            None => Err(Error::NoGame),
        }
    }

}

impl Serialize for Room {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("room", 8)?;
        state.serialize_field("id", self.id())?;
        state.serialize_field("name", self.name())?;
        state.serialize_field("is_public", &self.is_public)?;
        state.serialize_field("password", self.password() )?;
        state.serialize_field("owner", self.owner())?;
        state.serialize_field("max_players", self.max_players())?;
        state.serialize_field("players", &*executor::block_on(self.players.read()))?;
        state.serialize_field("game", &self.game().is_some())?;
        state.end()
    }
}


pub struct Partial(pub Room);
impl Serialize for Partial {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("room", 8)?;
        state.serialize_field("id", self.0.id())?;
        state.serialize_field("name", self.0.name())?;
        state.serialize_field("is_public", &self.0.is_public)?;
        state.serialize_field("password", &self.0.password().is_some() )?;
        state.serialize_field("owner", self.0.owner())?;
        state.serialize_field("max_players", self.0.max_players())?;
        state.serialize_field("players", &executor::block_on(self.0.players.read()).len())?;
        state.serialize_field("game", &self.0.game().is_some())?;
        state.end()
    }
}

impl TableEvents for Room {
    fn insert(&self) {
        let content = Payload::RoomCreate(self.clone()).to_json_string();
        self.announce(content)
    }

    fn update(&self) {
        let content = Payload::RoomUpdate(self.clone()).to_json_string();
        self.announce(content)
    }

    fn delete(&self) {
        let content = Payload::RoomDelete(self.id().clone()).to_json_string();
        self.announce(content)
    }
}

impl Borrow<String> for Room {
    fn borrow(&self) -> &String {
        &self.id()
    }
}
impl Borrow<Uuid> for Room {
    fn borrow(&self) -> &Uuid {
        &self.owner()
    }
}

impl Eq for Room { }

impl PartialEq for Room
 {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Hash for Room
 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}
