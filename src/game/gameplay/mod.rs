pub mod card;
pub mod player;

use std::collections::HashSet;
use sea_orm::prelude::Uuid;
use serde::Serialize;
use crate::{game::rooms, gateway::payloads::Payload};
use card::{ Card, Element, Effect };
use player::*;
use tokio::sync::broadcast::Sender;

pub enum Ok { //Список усіх можливих Ok значень
    Ok,
    GameOver(Losers),
}

#[derive(Debug, Serialize)]
pub enum Error { //Список усіх можливих помилок
    NotEnoughPlayers,
    CardNotFound,
    PlayerNotFound,
    WrongTurn,
    WrongCard,
    NoCardsLeft,
}

#[derive(Debug, Serialize, Clone)]
enum Direction { //Список напрямків ходів
    Next,
    Previous,
}

impl Direction {
    fn switch(&mut self) -> &mut Self { //перемикач напрямку
        *self = match self {
            Direction::Next => Direction::Previous,
            Direction::Previous => Direction::Next,
        };
        self
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Game { //Структура, що описує гру
    card: Card, //Попередня зіграна карта
    players: Vec<Player>, //Вектор гравців
    turn: usize, //індекс гравця, що ходить
    direction: Direction, //напрямок ходів
    #[serde(skip)] //пропуск серіалізації
    losers: Vec<Loser>, //вектор гравців, що вибули
    #[serde(skip)]
    gameover: bool, //чи закінчилася ця гра
}

impl Game {
    pub fn new(players: HashSet<rooms::player::Player>) -> Result<Self, Error> { //Конструктор
        let mut players_new = Vec::new(); //створення нового вектору
        for player in players.into_iter().filter(|player| player.is_ready == true) { //Перетворення набору гравців кімнати у вектор гравців гри,
            //беруться тільки готові гравці
            players_new.push(player.into()) //перетворення додавання гравця у вектор
        };
        if players_new.len() < 2 { return Err(Error::NotEnoughPlayers) } //якщо гравців менше за 2, то гра не може бути розпочатою. Повернення помилки
        Ok(
            Self {
                card: Card::new(Element::Energy, Effect::Flow),
                players: players_new,
                turn: 0,
                direction: Direction::Next,
                losers: Vec::new(),
                gameover: false,
            }
        )
    }


    pub fn announce(&self, content: String ) { //Оголошення повідомлення
        let game = self.clone();
        tokio::spawn(async move {
            for player in game.players {
                let _ = player.sender.send(content.clone());
            }
        });
    }

    pub fn player_update_sender(&mut self, player_id: Uuid, sender: Sender<String>) -> bool { //Відновлення сесії гравця
        let Ok(index) = self.get_player_index(player_id) else { return false };
        let game = self.clone();
        if let Some(player) = self.players.get_mut(index) {
            player.sender = sender.clone();
            let _ = sender.send(Payload::GameStarted(game.clone()).to_json_string());
            let _ = sender.send(Payload::GamePlayerCards(player.cards().clone()).to_json_string());
            true
        } else { false }
    }

    pub fn announce_turn(&self, new_turn: bool) { //Оголошення нового ходу
        let game = self.clone();
        tokio::spawn(async move { //Створення нового потоку, щоб не тормозити гру
            for player in game.players.iter() {
                if new_turn == true { //Якщо не нова гра, то надіслати інформацію про новий хід
                    let _ = player.sender.send(Payload::GameNewTurn(game.clone()).to_json_string());
                }
                let cards = player.cards().clone(); //Доставання карт гравця
                let _ = player.sender.send(Payload::GamePlayerCards(cards).to_json_string()); //надсилання інформації про наявні карти відповідному гравцю
            }
        });
    }

    pub fn get_player_index(&mut self, player_id: Uuid) -> Result<usize, Error> { //метод для знаходження індексу гравця у векторі за його id
        self.players.iter().enumerate() //пройтися по вектору та прономерувати
            .find(|(_index, player)| { *player.id() == player_id }) //Знайти гравця з вказаним id
            .ok_or(Error::PlayerNotFound) //видати помилку якщо не знайдено
            .map(|player| player.0) //дістати індекс з tuple та повернути
    }

    pub fn gameover(&self) -> &bool { //Геттер значення поля gameover
        &self.gameover
    }

    pub fn play(&mut self, player_id: Uuid, card_id: Option<usize>) -> Result<Ok, Error> { //Метод для грання
        let mut step = 1; //Наступний хід буде належати гравцю з індексом теперішнього ходу + або - цієї змінної в залежності напрямку
        let index = self.get_player_index(player_id)?; //доставання індексу гравця, що грає
        if index != self.turn { return Err(Error::WrongTurn) } //якщо не його хід то повернути помилку
        let player = &mut self.players[index]; //звертання до об'єкту гравця за індексом
        let mut cards_to_pick = 0; //кількість карт які наступний гравець повинен буде взяти
        if let Some(card_id) = card_id { //якщо вказаний індекс карти
            let card = player.get_card(card_id).ok_or(Error::CardNotFound)?; //доставання посилання на карту гравця за вказаним індексом
            let effect = card.play(self.card.clone()).map_err(|_| Error::WrongCard)?; //якщо індекс невірний то повернути помилку
            self.card = card.clone(); //клонування об'єкту
            player.remove_card(card_id); //Видалення цієї карти з вектору карт гравця
            match effect { //В залежності від ефекту:
                Effect::Stun => { step += 1 }, //Stun - Пропустити хід наступному гравцю
                Effect::Flow => { self.direction.switch(); }, //Flow - змінити напрямок
                Effect::Add(num) => {  //Add - додати наступному гравцю num карт
                    cards_to_pick = num;
                },
                _ => {}, //Будь-який інший - нічого
            }
            if player.cards().len() == 0 { //вибути з гри якщо не залишилося карт
                self.losers.push(self.players[index].clone().into()); 
                self.players.remove(index);
            }
        } else {
            let _ = self.pick_card(index); //якщо не вказаний індекс карти, то взяти з колоди
        }
        for _i in 0..cards_to_pick {
            let next = self.turn + match self.direction {
                Direction::Next => 1,
                Direction::Previous => -1,
            };
            let _ = self.pick_card(next);
        }
        let turn = match self.direction { //визначення хто ходить наступним за напрямком
            Direction::Next => self.turn as isize + step,
            Direction::Previous => self.turn as isize - step,
        };
        //Зациклення результату таким чином, щоб він завжди був в межах довжини вектора
        if turn < 0 {
            self.turn = (turn + self.players.len() as isize) as usize;
        } else if turn >= self.players.len() as isize {
            self.turn = (turn - self.players.len() as isize) as usize;
        } else {
            self.turn = turn as usize;
        }
        if self.players.len() <= 1 { //Якщо залишився тільки один гравець
            //то завершити гру 
            if let Some(winner) = self.players.pop() {
                self.losers.push(winner.into()); //додаючи останнього гравця в список вибувших
            }
            self.gameover = true;
            return Ok(Ok::GameOver(self.losers.clone().into())) //та повернути результат, що вказує на завершення
        }
        //Оголошення нового ходу
        self.announce_turn(true);
        Ok(Ok::Ok)
    }

    pub fn pick_card(&mut self, player_index: usize) -> Result<(), Error> { //Метод взяття карти з колоди
        //взяття гравця за індексом та повернення помилки якщо такого гравця немає
        let player = self.players.get_mut(player_index).ok_or(Error::PlayerNotFound)?;
        player.add_card(rand::random()); //Виклик методу додавання карти гравцю
        Ok(())
    }
}