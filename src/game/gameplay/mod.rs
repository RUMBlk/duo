pub mod card;
pub mod player;

use std::collections::HashSet;
use sea_orm::prelude::Uuid;
use serde::Serialize;
use crate::{game::rooms, gateway::payloads::Payload};
use card::{ Card, Element, Effect };
use player::Player;

#[derive(Debug)]
pub enum Error {
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

#[derive(Debug, Serialize, Clone)]
pub struct Game {
    card: Card,
    #[serde(skip)]
    cards: Vec<Card>,
    players: Vec<Player>,
    turn: usize,
    direction: Direction,
    #[serde(skip)]
    losers: u8,
}

impl Game {
    pub fn new(players: HashSet<rooms::player::Player>) -> Self {
        let mut cards: Vec<Card> = Vec::new();
        for _i in 0..players.len()*8*3 {
            cards.push(rand::random())
        }
        let mut players_new = Vec::new();
        for player in players.into_iter() {
            let mut player: Player = player.into();
            for _i in 0..8 {
                player.cards.push(rand::random())
            }
            players_new.push(player)
        };
        Self {
            card: Card::new(Element::Energy, Effect::Flow),
            cards,
            players: players_new,
            turn: 0,
            direction: Direction::Next,
            losers: 0,
        }
    }


    pub fn announce(&self, content: String ) {
        let game = self.clone();
        tokio::spawn(async move {
            for player in game.players {
                let _ = player.sender.send(content.clone());
            }
        });
    }

    pub fn announce_turn(&self) {
        let game = self.clone();
        tokio::spawn(async move {
            for player in game.players.iter() {
                let _ = player.sender.send(Payload::GameNewTurn(game.clone()).to_json_string());
                let cards = player.cards.clone();
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

    pub fn play(&mut self, player_id: Uuid, card_id: Option<usize>) -> Result<(), Error> {
        let mut step = 1;
        let index = self.get_player_index(player_id)?;
        if index != self.turn { return Err(Error::WrongTurn) }
        let player = &mut self.players[index];
        if let Some(card_id) = card_id {
            let card = player.cards.get(card_id).ok_or(Error::CardNotFound)?;
            let effect = card.play(self.card.clone()).map_err(|_| Error::WrongCard)?;
            self.card = card.clone();
            player.cards.remove(card_id);
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
            let num_of_cards = player.cards.len();
            if let Err(Error::NoCardsLeft) = self.pick_card(index) {
                if num_of_cards == 0 { 
                    self.losers += 1;
                    self.players[index].set_place(self.losers);
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
        Ok(())
    }

    pub fn pick_card(&mut self, player_index: usize) -> Result<(), Error> {
        let player = self.players.get_mut(player_index).ok_or(Error::PlayerNotFound)?;
        let card = self.cards.pop().ok_or(Error::NoCardsLeft)?;
        player.cards.push(card);
        Ok(())
    }
}