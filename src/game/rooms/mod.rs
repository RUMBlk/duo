pub mod player;

use std::{borrow::Borrow, collections::HashSet, hash::Hash};
use sea_orm::prelude::Uuid;
use tokio::sync::broadcast::Sender;
use random_string;
use serde::{ser::SerializeStruct, Serialize};
use player::Player;

#[derive(Debug, Serialize)]
pub enum Error<'a> {
    PlayerNotInRoom,
    BadArgument(&'a str),
    Forbidden(&'a str),
    CantAssignNewOwner,
    Full,
}

#[derive(Debug, Clone, Serialize)]
pub struct Room {
    id: String,
    name: String,
    pub is_public: bool,
    password: Option<String>,
    owner: Uuid,
    max_players: usize,
    players: HashSet<Player>,
    //pub game: Option<to implement>,
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
            players: HashSet::new(),
        }
    }
}

impl Room
 {
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

    pub fn players(&self) -> &HashSet<Player> {
        &self.players
    }

    pub fn players_mut(&mut self) -> &mut HashSet<Player> {
        &mut self.players
    }
}

pub trait Interaction<'a, 'b> {
    fn set_name(&'a mut self, name: String) -> Result<(), Error<'b>>;
    fn set_password(&'a mut self, password: Option<String>) -> Result<(), Error<'b>>;
    fn set_owner(&'a mut self, owner: Uuid) -> Result<(), Error<'b>>;
    fn set_max_players(&'a mut self, max_players: usize) -> Result<(), Error<'b>>;
    fn join(&'a mut self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>>;
    fn leave(&'a mut self, player_id: Uuid) -> Result<(), Error<'b>>;
    fn player_switch_ready(&'a mut self, player_id: Uuid) -> Result<(), Error<'b>>;
}

impl<'a, 'b> Interaction<'a, 'b> for Room {
    fn set_name(&mut self, name: String) -> Result<(), Error<'b>> {
        if !name.is_empty() { self.name = name; }
        else { return Err(Error::BadArgument("name can't be an empty string")) }
        Ok(())
    }

    fn set_password(&mut self, password: Option<String>) -> Result<(), Error<'b>> {
        if let Some(ref pass) = password {
            if pass.len() < 32 { 
                self.password = if pass.is_empty() { None } else { password }
            }
            else { return Err(Error::BadArgument("password can't be longer than 32 characters")) }
        }
        Ok(())
    }

    fn set_owner(&mut self, owner: Uuid) -> Result<(), Error<'b>> {
        self.owner = owner;
        Ok(())
    }

    fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> {
        if max_players < 2 { return Err( Error::BadArgument("max_players can't be lower than 2") ) }
        else { self.max_players = max_players }
        Ok(())
    }

    fn join(&mut self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> {
        if let Some(pass) = &self.password {
            if Some(pass) != password.as_ref() {
                return Err(Error::Forbidden("Wrong password"));
            }
        }
        if self.players.len() >= self.max_players {
            return Err(Error::Full)
        }
        

        //if self.players.contains(&player) { return Err(Error::PlayerAlreadyInRoom) }
        self.players.insert(player::Player::new(player_id, sender));
        Ok(())
    }

    fn leave(&mut self, player_id: Uuid) -> Result<(), Error<'b>> {
        if !self.players_mut().remove(&player_id) {
            return Err(Error::PlayerNotInRoom);
        };
        if self.owner == player_id {
            self.owner = self.players.iter().next().ok_or(Error::CantAssignNewOwner)?.id;
        }
        Ok(())
    }

    fn player_switch_ready(&'a mut self, player_id: Uuid) -> Result<(), Error<'b>> {
        let mut player = self.players.get(&player_id).ok_or(Error::PlayerNotInRoom)?.clone();
        player.is_ready = !player.is_ready;
        self.players.insert(player);
        Ok(())
    }
}

impl Borrow<String> for Room {
    fn borrow(&self) -> &String {
        &self.id
    }
}
impl Borrow<Uuid> for Room {
    fn borrow(&self) -> &Uuid {
        &self.owner
    }
}

impl Eq for Room { }

impl PartialEq for Room
 {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Room
 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct Partial(pub Room);
impl Serialize for Partial {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
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