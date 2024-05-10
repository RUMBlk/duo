use serde::{ Serialize, Deserialize };
use serde_json;
use sea_orm::prelude::Uuid;
use crate::game::rooms::{self, Room};
use sea_orm::prelude::DateTimeWithTimeZone;
use crate::database::entities;

#[derive(Debug, Serialize, Deserialize)]
pub enum Payload {
    //From Server
    OK,
    Error(Error),
    Hello(Hello),
    RoomReturn(RoomReturn),
    RoomNewPlayer(RoomNewPlayer),
    RoomPlayerLeft(RoomPlayerLeft),
    //From Server/Client
    Identify(Identify),
    Identity(Identity),
    RoomCreate(RoomCreate),
    RoomUpdate(RoomUpdate),
    //From Client
    RoomJoin(RoomJoin),
    RoomLeave(String),
}

impl Payload {
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize Gateway Payload")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
        BadRequest,
        Declined,
        BadToken,
        InvalidToken,
        InternalServerError,
        NotFound,
        Forbidden,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hello {
    heartbeat_interval: u64, 
}

impl Hello {
    pub fn new(heartbeat_interval: u64) -> Self {
        Self { heartbeat_interval }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identify {
    token: String,
}

impl Identify {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    uuid: Uuid,
    login: String,
    display_name: String,
    created_at: DateTimeWithTimeZone,
}

impl Identity {
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl From<entities::accounts::Model> for Identity {
    fn from(model: entities::accounts::Model) -> Self {
        let uuid = model.uuid;
        let login = model.login;
        let display_name = model.display_name;
        let created_at = model.created_at;
        Self { uuid, login, display_name, created_at }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomForm {
    name: Option<String>,
    is_public: Option<bool>,
    password: Option<String>,
    owner: Option<Uuid>,
    max_players: Option<usize>,
}

impl From<Room> for RoomForm {
    fn from(value: Room) -> Self {
        Self {
            name: Some(value.name()),
            is_public: Some(value.is_public),
            password: value.password(),
            owner: Some(value.owner()),
            max_players: Some(value.max_players()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomCreate {
    id: Option<String>,
    #[serde(flatten)]
    room_form: RoomForm,
}

impl RoomCreate {
    pub fn from_room(id: String, room: Room) -> Self {
        Self {
            id: Some(id),
            room_form: RoomForm::from(room),
        }
    }

    pub fn create_room(&self, owner: Uuid) -> Room {
        Room::new(
            self.room_form.name.clone(),
            Some(self.room_form.is_public.unwrap_or(false)),
            self.room_form.password.clone(),
            owner,
            self.room_form.max_players
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomUpdate {
    id: String,
    #[serde(flatten)]
    room_form: RoomForm,
}

impl RoomUpdate {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn apply_update(&mut self, room: &mut Room) -> RoomReturn {
        let mut applied = self.clone();
        let name = self.room_form.name.as_ref().and_then(|name| {
            Some(room.set_name(name.clone()))
        });
        if let Some(Err(_)) = name { applied.room_form.name = None };
        let is_public = self.room_form.is_public.and_then(|is_public| {
            room.is_public = is_public;
            Some(Ok(rooms::ReturnCode::OK))
        });
        let password = self.room_form.password.as_ref().and_then(|password| {
            let password = if password.is_empty() {
                None
            } else { Some(password) };
            Some(room.set_password(password.cloned()))
        });
        if let Some(Err(_)) = password { applied.room_form.password = None };
        let owner = self.room_form.owner.and_then(|player_id| {
            Some(room.set_owner(player_id))
        });
        if let Some(Err(_)) = owner { applied.room_form.owner = None };
        let max_players = self.room_form.max_players.and_then(|max_players| {
            Some(room.set_max_players(max_players))
        });
        if let Some(Err(_)) = max_players { applied.room_form.max_players = None };
        room.announce(Payload::RoomUpdate(applied).to_json_string());
        RoomReturn::new(name, is_public, password, owner, max_players)
    }
}


type RoomResult = Option<Result<rooms::ReturnCode, rooms::ReturnCode>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomReturn {
    name: RoomResult,
    is_public: RoomResult,
    password: RoomResult,
    owner: RoomResult,
    max_players: RoomResult,
}

impl RoomReturn {
    pub fn new(
        name: RoomResult,
        is_public: RoomResult,
        password: RoomResult,
        owner: RoomResult,
        max_players: RoomResult,
    ) -> Self {
        Self {
            name,
            is_public,
            password,
            owner,
            max_players,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomJoin {
    id: String,
    password: Option<String>,
}

impl RoomJoin {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomNewPlayer {
    room_id: String,
    player_id: Uuid,
}

impl RoomNewPlayer {
    pub fn new(room_id: String, player_id: Uuid) -> Self {
        Self {
            room_id,
            player_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomPlayerLeft {
    room_id: String,
    player_id: Uuid,
}

impl RoomPlayerLeft {
    pub fn new(room_id: String, player_id: Uuid) -> Self {
        Self {
            room_id,
            player_id,
        }
    }
}