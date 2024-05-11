use serde::{ Serialize, Deserialize };
use sea_orm::prelude::Uuid;
use std::collections::HashMap;
use tokio::sync::broadcast::{ self, Sender };
use random_string;

#[derive(Debug, Serialize, Deserialize)]
pub enum ReturnCode {
    OK,
    PlayerAlreadyInRoom,
    PlayerNotInRoom,
    InvalidName,
    InvalidPassword,
    NoOwner,
    Full,
    MaxPlayersNotSet,
    MaxPlayersCantBeLowerThan(usize),
}

#[derive(Debug, Serialize, Clone)]
pub struct Player {
    #[serde(skip_serializing)]
    sender: broadcast::Sender<String>,
    is_ready: bool,
    points: u64,
}

impl Player {
    pub fn new(sender: Sender<String>) -> Self {
        Self { sender, is_ready: false, points: 0 }
    }
}

pub type Table = HashMap::<String, Room>;
pub type Players = HashMap<Uuid, Player>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Room {
    name: Option<String>,
    pub is_public: Option<bool>,
    password: Option<String>,
    owner: Option<Uuid>,
    max_players: Option<usize>,
    #[serde(skip_deserializing)]
    players: Players,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            name: Some(String::from("Room")),
            is_public: Some(false),
            password: None,
            owner: None,
            max_players: Some(2),
            players: HashMap::new(),
        }
    }
}

impl Room {
    pub fn new(name: Option<String>, is_public: Option<bool>, password: Option<String>, owner: Option<Uuid>, max_players: Option<usize>) -> Self {
        Self { 
            name,
            is_public,
            password,
            owner,
            max_players,
            players: HashMap::new()
        }
    }

    pub fn generate_id(&self) -> String {
        random_string::generate(6, "0123456789")
    }

    pub fn announce(&self, text: String) {
        for player in self.players.iter() {
            let _ = player.1.sender.send(text.clone());
        }
    }

    pub fn announce_self(&self) {
        self.announce(serde_json::to_string(self).expect("Failed to serialize Room"))
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn password(&self) -> &Option<String> {
        &self.password
    }

    pub fn owner(&self) -> &Option<Uuid> {
        &self.owner
    }

    pub fn max_players(&self) -> &Option<usize> {
        &self.max_players
    }

    pub fn set_name(&mut self, name: String) -> Result<ReturnCode, ReturnCode> {
        if !name.is_empty() { self.name = Some(name); }
        else { return Err(ReturnCode::InvalidName) }
        Ok(ReturnCode::OK)
    }

    pub fn set_password(&mut self, password: Option<String>) -> Result<ReturnCode, ReturnCode> {
        if let Some(ref pass) = password {
            if pass.len() < 32 { 
                self.password = if pass.is_empty() { None } else { password }
            }
            else { return Err(ReturnCode::InvalidPassword) }
        }
        Ok(ReturnCode::OK)
    }

    pub fn set_owner(&mut self, player_id: Uuid) -> Result<ReturnCode, ReturnCode> {
        if self.players.contains_key(&player_id) { self.owner = Some(player_id); }
        else { return Err(ReturnCode::PlayerNotInRoom) };
        Ok(ReturnCode::OK)
    }

    pub fn set_max_players(&mut self, max_players: usize) -> Result<ReturnCode, ReturnCode> {
        if max_players < 2 { return Err( ReturnCode::MaxPlayersCantBeLowerThan(Self::default().max_players.unwrap()) ) }
        else { self.max_players = Some(max_players) }
        Ok(ReturnCode::OK)
    }

    pub fn batch_update(&mut self, from: &Self) -> Vec<Result<ReturnCode, ReturnCode>> {
        let mut errors = Vec::new();
        if let Some(value) = &from.name { errors.push( self.set_name(value.to_owned())) };
        if let Some(value) = from.is_public { self.is_public = Some(value); };
        errors.push( self.set_password(from.password.clone()));
        if let Some(value) = from.owner { errors.push( self.set_owner(value)) };
        if let Some(value) = from.max_players { errors.push( self.set_max_players(value)) };
        self.announce_self();
        errors
    }

    pub fn join(&mut self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<ReturnCode, ReturnCode> {
        if let Some(pass) = &self.password {
            if Some(pass) != password.as_ref() {
                return Err(ReturnCode::InvalidPassword);
            }
        }
        let max_players = self.max_players.ok_or(ReturnCode::MaxPlayersNotSet)?;
        if self.players.len() >= max_players {
            return Err(ReturnCode::Full)
        }

        self.players.insert(player_id, Player::new(sender)).ok_or(ReturnCode::PlayerAlreadyInRoom)?;
        Ok(ReturnCode::OK)
    }

    pub fn leave(&mut self, player_id: Uuid) -> Result<ReturnCode, ReturnCode> {
        self.players.remove(&player_id).ok_or(ReturnCode::PlayerNotInRoom)?;
        if self.owner == Some(player_id) { 
            let next_owner = self.players.iter().next()
                .ok_or(ReturnCode::NoOwner)?;
            self.owner = Some(*next_owner.0);
        }
        Ok(ReturnCode::OK)
    }

    pub fn contains_player(&self, player_id: Uuid) -> bool {
        self.players.contains_key(&player_id)
    }

    pub fn number_of_players(&self) -> usize {
        self.players.len()
    }

    pub fn get_partial(&self) -> Self {
        let mut room = self.clone();
        room.players = HashMap::new();
        room
    }

}