use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Element {
    Water,
    Fire,
    Wood,
    Earth,
    Air,
    Energy,
}

impl Element {
    pub fn index(&self) -> usize {
        *self as usize
    }

    //self - current played card
    //other - previous played card
    pub fn coefficient(&self, other: Self) -> f32 {
        if *self == Element::Energy || other == Element::Energy { return 1.0 };
        let pos = self.index() as isize;
        let other_pos = other.index() as isize;
        let distance = if pos <= other_pos {
            let mut distance = other_pos - pos; 
            let half = (Element::Energy as isize - 1)/2;
            if distance > half { distance += 1 }
            else if distance == half { return 1.0 }
            distance
        } else {
            Element::Energy.index() as isize + other_pos - pos
        };
        if distance == 0 { return 1.0 }
        0.50 + (Element::Energy as isize - distance) as f32 / 4_f32
    }
}

impl Distribution<Element> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Element {
        match rng.gen_range(0..=5) {
            0 => Element::Water,
            1 => Element::Fire,
            2 => Element::Wood,
            3 => Element::Earth,
            4 => Element::Air,
            _ => Element::Energy,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Effect {
    Atk(u8),
    Flow,
    Stun,
    Add(u8),
}

impl Distribution<Effect> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Effect {
        match rng.gen_range(0..=3) {
            0 => Effect::Atk(rng.gen_range(1..=12)),
            1 => Effect::Flow,
            2 => Effect::Stun,
            _ => Effect::Add(rng.gen_range(1..=4)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    element: Element,
    effect: Effect, 
}

impl Card {
    pub fn new(element: Element, effect: Effect) -> Self {
        Self {
            element,
            effect,
        }
    }

    pub fn play(&self, card: Self) -> Result<Effect, ()> {
        let coef = self.element.coefficient(card.element);
        let other_power = match card.effect {
            Effect::Atk(power) => power,
            _ => 1,
        };
        match self.effect {
            Effect::Atk(power) => {
                if (power as f32 *coef).round() < other_power as f32 { return Err(()) }
                Ok(Effect::Atk(power))
            },
            effect => {
                if coef < 1.0 { return Err(()) }
                Ok(effect)
            },
        }
    }
}

impl Distribution<Card> for Standard {
    fn sample<R: Rng + ?Sized>(&self, _rng: &mut R) -> Card {
        Card {
            element: rand::random(),
            effect: rand::random(), 
        }

    }
}