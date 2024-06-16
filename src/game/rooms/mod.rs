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
pub enum Error<'a> { //Список усіх помилок кімнат
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
pub struct Room { //Структура, що описує кімнату
    id: String, //Ідентифікатор
    name: String, //Назва
    pub is_public: bool, //Видимість в списку кімнат
    password: Option<String>, //Пароль
    owner: Uuid, //Власник
    max_players: usize, //Максимальна кількість гравців
    players: Arc<RwLock<DataTable<Player>>>, //Посилання на таблицю гравців кімнати
    pub game: Option<Arc<RwLock<Game>>>, //посилання на об'єкт гри
}

impl Default for Room
 {
    fn default() -> Self { //Значення за замовченням
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
    //Метод створення кімнати
    pub async fn create(name: String, is_public: bool, password: Option<String>, owner: Uuid, max_players: usize, sender: Sender<String>) -> Result<Self, Error<'b>> {
        let mut room = Self::default(); //Створення нової кімнати з значеннями за замовченням
        room.set_name(name)?; //задання назви,
        room.is_public = is_public; //публічності,
        room.set_password(password.clone())?; //пароля,
        room.set_max_players(max_players)?; //макс. кільк. гравців,
        room.join(password, owner.clone(), sender).await?; //приєднання створювача кімнати,
        room.set_owner(owner.clone())?; //назначення його власником
        //при помилці будь-якого сеттера, створення завершиться помилкою, інакше метод повертає екземпляр
        Ok(room)
    }

    //Оголошення повідомлення
    pub fn announce(&self, content: String ) {
        let room = self.clone();
        tokio::spawn(async move { //виділення потоку на кожне повідомлення
            for player in &**room.players().read().await {
                let _ = player.sender.send(content.clone());
            }
        });
    }

    pub fn generate_id() -> String { //Метод генерування ідентифікатора, що складається з 6 чисел
        random_string::generate(6, "0123456789")
    }

    pub fn regenerate_id(&mut self) { //Метод генерування нового ідентифікатора та його назначення
        self.id = Self::generate_id()
    }

    pub fn id(&self) -> &String { //Геттер ідентифікатора
        &self.id
    }

    pub fn name(&self) -> &String { //Геттер назви
        &self.name
    }

    pub fn password(&self) -> &Option<String> { //Геттер пароля
        &self.password
    }

    pub fn owner(&self) -> &Uuid { //Геттер власника
        &self.owner
    }

    pub fn max_players(&self) -> &usize { //Геттер макс. кільк. гравців
        &self.max_players
    }

    pub fn players(&self) -> &Arc<RwLock<DataTable<Player>>> { //Геттер посилання на таблицю гравців
        &self.players
    }

    pub fn game(&self) -> &Option<Arc<RwLock<Game>>> { //Геттер посилання на гру
        &self.game
    }

    pub fn set_name(&mut self, name: String) -> Result<(), Error<'b>> { //Сеттер назви
        if !name.is_empty() { self.name = name; } //якщо рядок name не пустий то назначити його у якості нової назви
        else { return Err(Error::BadArgument("name can't be an empty string")) } //інакше повернути помилку
        Ok(())
    }

    pub fn set_password(&mut self, password: Option<String>) -> Result<(), Error<'b>> { //Сеттер пароля
        if let Some(ref pass) = password { //якщо пароль не None(Null мови програмування Rust)
            if pass.len() <= 32 {  //якщо довжина <= 32
                self.password = if pass.is_empty() { None } else { password } //Якщо рядок пустий то пароля немає інакше назначити рядок новим паролем
            }
            else { return Err(Error::BadArgument("password can't be longer than 32 characters")) } //якщо пароль більший то видати помилку
        }
        Ok(())
    }

    pub fn set_owner(&mut self, owner: Uuid) -> Result<(), Error<'b>> { //Сеттер власника
        self.owner = owner;
        Ok(())
    }

    pub fn set_max_players(&mut self, max_players: usize) -> Result<(), Error<'b>> { //Сеттер макс. кільк. гравців
        if max_players < 2 { return Err( Error::BadArgument("max_players can't be lower than 2") ) }
        else { self.max_players = max_players }
        Ok(())
    }

    //Метод приєднання гравця до кімнати
    pub async fn join(&'a self, password: Option<String>, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> {
        let mut players = self.players.write().await; //Получення замка на зміну об'єкта за посиланням
        if let Some(pass) = &self.password { //якщо кімната має пароль
            if Some(pass) != password.as_ref() { //і якщо він не дорівнює вказаному в аргументі
                return Err(Error::Forbidden("Wrong password")); //то видати помилку
            }
        }
        if players.len() >= self.max_players && !players.contains(&player_id) { //якщо кількість гравців в кімнаті вже >= макс. кільк. гравців
            return Err(Error::Full) //то видати помилку
        }
        

        //if self.players.contains(&player) { return Err(Error::PlayerAlreadyInRoom) }
        players.shared_insert(player::Player::new(player_id, sender)); //Додати гравця до таблиці гравців кімнати
        Ok(())
    }

    //Метод виходу гравця з кімнати
    pub async fn leave(&'a mut self, player_id: Uuid) -> Result<bool, Error<'b>> {
        let mut players = self.players.write().await; //замок на зміну таблиці гравців
        if !players.shared_remove(&player_id) { //спробувати видалити вказаного гравця
            return Err(Error::PlayerNotInRoom); //якщо змін не відбулося значить гравця немає в кімнати, повернути помилку
        };
        let changed = self.owner == player_id; //порівняня власника і гравця, що вийшов
        if changed { //якщо власник вийшов
            self.owner = players.iter().next().ok_or(Error::CantAssignNewOwner)?.id; //назначити наступного гравця в таблиці
            //якщо гравців не залишилося, повернути помилку
        }
        Ok(changed)
    }

    pub async fn player_switch_ready(&'a self, player_id: Uuid) -> Result<(), Error<'b>> { //Метод перемикання готовності гравця
        if let Some(game) = &self.game {  //якщо кімната має розпочату гру
            if *game.read().await.gameover() == false { return Err(Error::GameAlreadyStarted); } //видати помилку якщо вона не завершена
        }
        let mut players = self.players.write().await; //замок на зміну таблиці гравців
        players.shared_update(&player_id, |player| { //виклик методу пошуку гравця за id,
            //при знайденому гравці передати його в це замикання
            player.is_ready = !player.is_ready; //перемкнути готовність
            Ok::<(), ()>(()) //повернути пусте значення
        }).unwrap_or(None).ok_or(Error::PlayerNotInRoom)?; //Повернути помилку якщо гравець не в кімнаті
        Ok(())
    }

    pub async fn player_update_sender(&'a self, player_id: Uuid, sender: Sender<String>) -> Result<(), Error<'b>> { //Метод відновлення сесії
        let mut players = self.players.write().await; //замок на зміну таблиці гравця
        players.shared_update(&player_id, |player| { //пошук гравця за id, передача в замикання
            player.sender = sender.clone(); //та назначення нового надсилача
            Ok::<(), ()>(())
        }).unwrap_or(None).ok_or(Error::PlayerNotInRoom)?; //повернути помилку якщо гравця немає в кімнаті
        drop(players); //відімкнути таблицю гравців, таким чином даючи можливість іншим потокам взаємодіяти з значенням за посиланням
        let _ = sender.send(Payload::RoomCreate(self.clone()).to_json_string()); //надіслати гравцю, що відновлює сесію інформацію про кімнату
        if let Some(game) = &self.game { //якщо кімната має розпочату гру
            let mut game = game.write().await; //замок
            game.player_update_sender(player_id, sender.clone()); //також відновити сесію гравця в грі
        }
        Ok(())
    }

    async fn new_game(&'a mut self) -> Result<(), Error<'b>> { //метод створення нової гри
        let game_obj = Game::new(self.players.read().await.deref().deref().clone())
            .map_err(|e| Error::Game(e))?; //Створення нового екземпляру гри, в якості аргумента надається клонована таблиця гравців кімнати
        self.game = Some(Arc::new(RwLock::new(game_obj.clone()))); //створення розумного вказівника та поміщення його в game
        let game = self.game.as_ref().unwrap().read().await; //замок на читання об'єкту гри
        game.announce(Payload::GameStarted(game_obj).to_json_string()); //оголошення створення нової гри
        game.announce_turn(false); //оголошення наявних карт гравцям
        Ok(())
    }
    pub async fn start_game(&'a mut self) -> Result<(), Error<'b>> { //метод створення нової гри тільки у випадку якщо вона ще не розпочалася
        match &self.game {
            Some(game) => { //якщо вже є розпочата гра 
                if *game.read().await.gameover() == true { //якщо вона закінчена
                    let _ = self.new_game().await; //створити нову
                    Ok(())
                } else {
                    Err(Error::GameAlreadyStarted) //інакше повернути помилку
                }
            },
            None => { //якщо гра не розпочалася
                let _ = self.new_game().await; //створити нову
                Ok(())
            }
        }
    }

    //Метод взаємодії з грою
    pub async fn play_game(&'a self, player_id: Uuid, card_id: Option<usize>) -> Result<Ok, Error<'b>> {
        match &self.game {
            Some(game) => { //якщо гра розпочалася
                let mut game =game.write().await; //замок на об'єкт гри
                let result = game.play(player_id, card_id).map_err(|e| Error::Game(e))?; //зіграти та зберегти результат
                match result {
                    Ok::GameOver(ref players ) => { //якщо гра завершилася
                        let mut room_players = self.players.write().await; //замок на гравців у кімнаті
                        for loser in players.iter() { //ітерація усих гравців гри
                            let _ = room_players.shared_update(loser.id(), |player| { //пошук гравців кімнати та оновлення їх значень
                                player.is_ready = false; //усі гравці повинні бути не готові після гри
                                player.points += loser.points(); //додавання очків з гри до очків з усих ігор
                                Ok::<(), ()>(())
                            });
                        };
                        self.announce(Payload::GameOver(players.clone()).to_json_string()); //оголошення кінця гри
                    },
                    _ => {},
                };
                Ok(result)
            },
            None => Err(Error::NoGame), //Повернення помилки у випадку якщо гра не розпочалася
        }
    }

}


//Власноруч реалізований серіалізатор 
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
        state.serialize_field("players", &*executor::block_on(self.players.read()))?; //показники не серіалізуються, саме тому власна реалізація необхідна
        state.serialize_field("game", &self.game().is_some())?;
        state.end()
    }
}


pub struct Partial(pub Room); //обгортка кімнати в нову структуру, серіалізація якої надає скорочену інформацію про кімнату
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
        state.serialize_field("players", &executor::block_on(self.0.players.read()).len())?; //а саме замість гравців серіалізується тільки їх кількість
        state.serialize_field("game", &self.0.game().is_some())?;
        state.end()
    }
}

//Реалізація Gateway подій для Room 
impl TableEvents for Room {
    fn insert(&self) {
        let content = Payload::RoomCreate(self.clone()).to_json_string(); //Сформувати новий Payload
        self.announce(content) //Оголосити його
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

impl Borrow<String> for Room { //При взятті показника Room у якості String надати показник на поле id
    fn borrow(&self) -> &String {
        &self.id()
    }
}
impl Borrow<Uuid> for Room { //При взятті показника Room у якості Uuid надати показник на поле owner
    fn borrow(&self) -> &Uuid {
        &self.owner()
    }
}

impl Eq for Room { } //Реалізація Eq ознаки за замовченням для Room

impl PartialEq for Room //Власна реалізація ознаки порівняння, таким чином щоб структура Room порівнювалась за полем id
{
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl Hash for Room //Реалізація ознаки Hash для Room, таким чином щоб хеш-функція обчислювалась тільки від поля id
 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}
