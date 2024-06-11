pub mod card;
pub mod player;

use std::collections::HashSet;
use sea_orm::prelude::Uuid;
use serde::{ser::SerializeStruct, Serialize};
use crate::{game::rooms, gateway::payloads::Payload};
use card::{ Card, Element, Effect };
use player::*;
use tokio::sync::broadcast::Sender;

pub enum Ok {
    Ok,
    GameOver(Losers),
}

#[derive(Debug, Serialize)]
pub enum Error {
    NotEnoughPlayers,
    CardNotFound,
    PlayerNotFound,
    WrongTurn,
    WrongCard,
    NoCardsLeft,
}

#[derive(Debug, Serialize, Clone)]
enum Direction {
    Next,
    Previous,
}

impl Direction {
    fn switch(&mut self) -> &mut Self {
        *self = match self {
            Direction::Next => Direction::Previous,
            Direction::Previous => Direction::Next,
        };
        self
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    card: Card,
    cards: Vec<Card>,
    players: Vec<Player>,
    turn: usize,
    direction: Direction,
    losers: Vec<Loser>,
}

impl Game {
    pub fn new(players: HashSet<rooms::player::Player>) -> Result<Self, Error> {
        let mut cards: Vec<Card> = Vec::new();
        for _i in 0..players.len()*8*3 {
            cards.push(rand::random())
        }
        let mut players_new = Vec::new();
        for player in players.into_iter().filter(|player| player.is_ready == true) {
            let mut player: Player = player.into();
            for _i in 0..8 {
                player.add_card(rand::random())
            }
            players_new.push(player)
        };
        if players_new.len() < 2 { return Err(Error::NotEnoughPlayers) }
        Ok(
            Self {
                card: Card::new(Element::Energy, Effect::Flow),
                cards,
                players: players_new,
                turn: 0,
                direction: Direction::Next,
                losers: Vec::new(),
            }
        )
    }


    pub fn announce(&self, content: String ) {
        let game = self.clone();
        tokio::spawn(async move {
            for player in game.players {
                let _ = player.sender.send(content.clone());
            }
        });
    }

    pub fn player_update_sender(&mut self, player_id: Uuid, sender: Sender<String>) -> bool {
        let Ok(index) = self.get_player_index(player_id) else { return false };
        let game = self.clone();
        if let Some(player) = self.players.get_mut(index) {
            player.sender = sender.clone();
            let _ = sender.send(Payload::GameStarted(game.clone()).to_json_string());
            let _ = sender.send(Payload::GamePlayerCards(player.cards().clone()).to_json_string());
            true
        } else { false }
    }

    pub fn announce_turn(&self) {
        let game = self.clone();
        tokio::spawn(async move {
            for player in game.players.iter() {
                let _ = player.sender.send(Payload::GameNewTurn(game.clone()).to_json_string());
                let cards = player.cards().clone();
                let _ = player.sender.send(Payload::GamePlayerCards(cards).to_json_string());
            }
        });
    }

    pub fn get_player_index(&mut self, player_id: Uuid) -> Result<usize, Error> {
        self.players.iter().enumerate()
            .find(|(_index, player)| { *player.id() == player_id })
            .ok_or(Error::PlayerNotFound)
            .map(|player| player.0)
    }

    pub fn play(&mut self, player_id: Uuid, card_id: Option<usize>) -> Result<Ok, Error> {
        let mut step = 1;
        let index = self.get_player_index(player_id)?;
        if index != self.turn { return Err(Error::WrongTurn) }
        let player = &mut self.players[index];
        if let Some(card_id) = card_id {
            let card = player.get_card(card_id).ok_or(Error::CardNotFound)?;
            let effect = card.play(self.card.clone()).map_err(|_| Error::WrongCard)?;
            self.card = card.clone();
            player.remove_card(card_id);
            match effect {
                Effect::Stun => { step += 1 },
                Effect::Flow => { self.direction.switch(); },
                Effect::Add(num) => { 
                    for _i in 0..num {
                        let _ = self.pick_card(self.turn+1);
                    }
                },
                _ => {},
            }
        } else {
            let num_of_cards = player.cards().len();
            if let Err(Error::NoCardsLeft) = self.pick_card(index) {
                if num_of_cards == 0 {
                    self.losers.push(self.players[index].clone().into()); 
                    self.players.remove(index);
                }
            }
        }
        let turn = match self.direction {
            Direction::Next => self.turn as isize + step,
            Direction::Previous => self.turn as isize - step,
        };
        if turn < 0 {
            self.turn = (turn + self.players.len() as isize) as usize;
        } else if turn >= self.players.len() as isize {
            self.turn = (turn - self.players.len() as isize) as usize;
        } else {
            self.turn = turn as usize;
        }
        self.announce_turn();
        if self.players.len() <= 1 { 
            if let Some(winner) = self.players.pop() {
                self.losers.push(winner.into());
            }
            return Ok(Ok::GameOver(self.losers.clone().into())) 
        }
        Ok(Ok::Ok)
    }

    pub fn pick_card(&mut self, player_index: usize) -> Result<(), Error> {
        let player = self.players.get_mut(player_index).ok_or(Error::PlayerNotFound)?;
        let card = self.cards.pop().ok_or(Error::NoCardsLeft)?;
        player.add_card(card);
        Ok(())
    }
}

impl Serialize for Game {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        let mut state = serializer.serialize_struct("Game", 5)?;
        state.serialize_field("card", &self.card)?;
        state.serialize_field("cards", &self.card.len())?;
        state.serialize_field("players", &self.players)?;
        state.serialize_field("turn", &self.turn)?;
        state.serialize_field("direction", &self.direction)?;
        state.end()
    }
}