use sea_orm::prelude::Uuid;
use serde::{ser::SerializeStruct, Serialize};
use std::{ borrow::Borrow, hash::Hash, ops::Deref };
use tokio::sync::broadcast::Sender;
use super::card::Card;
use crate::game::rooms;

#[derive(Debug, Clone)]
pub struct Player { //Структура, яка описує гравця гри
    id: Uuid, //Ідентифікатор
    pub sender: Sender<String>, //Надсилач, на який підписується веб-сокет та по якому відправляються усі повідомлення
    cards: Vec<Card>, //Вектор карт
    cards_count: u16, //Список карт які гравець тримав загалом
}

impl Player {
    pub fn id(&self) -> &Uuid { //Геттер ідентифікатора
        &self.id
    }

    pub fn add_card(&mut self, card: Card) { //Метод додавання карти
        self.cards.push(card);
        self.cards_count += 1;
    }

    pub fn remove_card(&mut self, index: usize) -> Card { //Метод видалення карти
        self.cards.remove(index)
    }

    pub fn get_card(&self, index: usize) -> Option<&Card> { //Метод взяття карти
        self.cards.get(index)
    }

    pub fn cards(&self) -> &Vec<Card> { //Геттер вектора карт
        &self.cards
    }
}

impl From<rooms::player::Player> for Player { //Ознака яка реалізує перетворення гравця кімнати у гравця гри
    fn from(value: rooms::player::Player) -> Self {
        let mut cards: Vec<Card> = Vec::new();
        for _i in 0..8 { //додавання восьми карт
            cards.push(rand::random());
        }
        Self {
            id: value.id,
            sender: value.sender,
            cards,
            cards_count: 0,
        }
    }
}

impl Hash for Player { //Налаштування хешування Player, його унікальність повинна визначатися тільки за id
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Eq for Player { } //Порівняння за id
impl PartialEq for Player {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Borrow<Uuid> for Player { //Реалізація показника на поле id
    fn borrow(&self) -> &Uuid {
        &self.id
    }
}

impl Serialize for Player { //Власноруч реалізована серіалізація
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("Player", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("cards", &self.cards.len())?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Loser { //Гравець, що вибув з гри
    id: Uuid, //Ідентифікатор
    points: u64, //Кількість очків
    cards_had: u16, //Кількість карт які гравець мав за гру загалом
}

impl Loser {
    pub fn new(id: Uuid) -> Self { //Конструктор
        Self { id, points: 0, cards_had: 0 }
    }

    pub fn id(&self) -> &Uuid { //Геттер ідентифікатора
        &self.id
    }

    pub fn points(&self) -> &u64 { //Геттер очків
        &self.points
    }

    pub fn cards_had(&self) -> &u16 { //Геттер кількості карт які гравець мав за гру загалом
        &self.cards_had
    }
}

impl From<Player> for Loser { //Перетворювач гравця в гравця, що вибув
    fn from(value: Player) -> Self {
        Self { id: value.id, points: 0, cards_had: value.cards_count }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Losers(Vec<Loser>); //Обертання вектора в нову структуру для додавання власних методів

impl From<Vec<Loser>> for Losers {
    fn from(value: Vec<Loser>) -> Self {
        //Цей перетворювач обчислює кількість очків гравців всередині вектора
        let len = value.len();
        let mut losers = Vec::new();
        for (i, loser) in value.iter().enumerate() {
            let mut loser = loser.clone();
            loser.points = ((len*10)*(len-i)/len) as u64;
            losers.push(loser);
        }
        Self(losers)
    }
}

impl Deref for Losers { //При зверненні до об'єкту Losers повертати Vec<Loser>
    type Target = Vec<Loser>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}