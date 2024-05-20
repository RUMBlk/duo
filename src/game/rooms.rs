use std::{borrow::Borrow, collections::HashSet, hash::Hash};
use tokio::sync::RwLock;
use random_string;
use serde::Serialize;
use std::sync::Arc;
use super::player::Player;

#[derive(Debug, Serialize)]
pub enum Error<'a> {
    PlayerAlreadyInRoom,
    PlayerNotInRoom,
    BadArgument(&'a str),
    Forbidden(&'a str),
    Full,
}

#[derive(Debug, Clone)]
pub struct Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    pub stored_in: Option<Arc<RwLock<HashSet<Room<PlayerData, Ownership>>>>>,
    id: String,
    name: String,
    pub is_public: bool,
    password: Option<String>,
    owner: Option<Ownership>,
    max_players: usize,
    players: HashSet<Player<PlayerData>>,
}

impl<PlayerData, Ownership> Borrow<String> for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn borrow(&self) -> &String {
        &self.id
    }
}
impl<PlayerData, Ownership> Borrow<Option<Ownership>> for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn borrow(&self) -> &Option<Ownership> {
        &self.owner
    }
}

impl<PlayerData, Ownership> Eq for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash { }

impl<PlayerData, Ownership> PartialEq for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<PlayerData, Ownership> Hash for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<PlayerData, Ownership> Default for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
    fn default() -> Self {
        Self {
            stored_in: None,
            id: Self::generate_id(),
            name: String::from("Room"),
            is_public: false,
            password: None,
            owner: None,
            max_players: 2,
            players: HashSet::new(),
        }
    }
}

impl<PlayerData, Ownership> Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash {
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

    pub fn owner(&self) -> &Option<Ownership> {
        &self.owner
    }

    pub fn max_players(&self) -> &usize {
        &self.max_players
    }

    pub fn players(&self) -> &HashSet<Player<PlayerData>> {
        &self.players
    }

    pub fn players_mut(&mut self) -> &mut HashSet<Player<PlayerData>> {
        &mut self.players
    }
}

pub trait Interaction<'a, 'b, PlayerData, Ownership> {
    fn set_name(&'a mut self, name: String) -> Result<(), Error<'b>>;
    fn set_password(&'a mut self, password: Option<String>) -> Result<(), Error<'b>>;
    fn set_owner(&'a mut self, ownership: Ownership) -> Result<(), Error<'b>>;
    fn set_max_players(&'a mut self, max_players: usize) -> Result<(), Error<'b>>;
    fn join(&'a mut self, password: Option<String>, player: PlayerData) -> Result<(), Error<'b>>;
    fn leave(&'a mut self, player: PlayerData) -> Result<(), Error<'b>>;
}

impl<'a, 'b, PlayerData, Ownership> Interaction<'a, 'b, PlayerData, Ownership> for Room<PlayerData, Ownership>
where PlayerData: PartialEq + std::cmp::Eq + Hash + Borrow<PlayerData> {
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

    fn set_owner(&mut self, ownership: Ownership) -> Result<(), Error<'b>> {
        self.owner = Some(ownership);
        Ok(())
    }

    fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> {
        if max_players < 2 { return Err( Error::BadArgument("max_players can't be lower than 2") ) }
        else { self.max_players = max_players }
        Ok(())
    }

    fn join(&mut self, password: Option<String>, player: PlayerData) -> Result<(), Error<'b>> {
        if let Some(pass) = &self.password {
            if Some(pass) != password.as_ref() {
                return Err(Error::Forbidden("Wrong password"));
            }
        }
        if self.players.len() >= self.max_players {
            return Err(Error::Full)
        }

        let player = Player::from(player);
        //if self.players.contains(&player) { return Err(Error::PlayerAlreadyInRoom) }
        self.players.insert(player);
        Ok(())
    }

    fn leave(&mut self, player: PlayerData) -> Result<(), Error<'b>> {
        if !self.players_mut().remove::<PlayerData>(&player) {
            return Err(Error::PlayerNotInRoom);
        };
        Ok(())
    }
}