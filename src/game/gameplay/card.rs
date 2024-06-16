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
    pub fn index(&self) -> usize { //Геттер позиції наданого елементу
        *self as usize
    }

    //self - карта зіграна щойно
    //other - карта зіграна попереднім гравцем
    pub fn coefficient(&self, other: Self) -> f32 {
        if *self == Element::Energy || other == Element::Energy { return 1.0 }; //Якщо елемент однієї з двох карток, то повернути коефіцієнт 1.0 
        //Визначення позицій карток
        let pos = self.index() as isize + 1;
        let other_pos = other.index() as isize + 1;
        //Визначення середньої позиції списку
        let half = (Element::Energy as isize - 1)/2;
        let mut distance = if pos <= other_pos { //якщо елемент карти зіграної щойно в списку стоїть до елементу карти зіграної попереднім гравцем
            other_pos - pos //то більше відняти за менше
        } else {
            Element::Energy.index() as isize + other_pos - pos //інакше зробити теж саме, але також додати індекс останнього елементу для забезпечення циклічності елементів
            //це необхідно так як елемент карти зіграної щойно розташований після елементу карти зіграної п.г і тому йому необхідно пройти до кінця списку,
            //почати з початку і тільки потім дійти  
        };
        if distance == 0 { return 1.0 } // якщо елементи однакові то повернути коефіцієнт 1.0
        if distance > half { distance += 1 } //ребалансування після втручення в формулу коефіцієнту в попередньому рядку
        else {
            distance -= 1;
        }
        0.50 + (Element::Energy as isize - distance) as f32 / 4_f32 //Повернення значення обчисленого за формулою для визначення коефіцієнту
    }
}

impl Distribution<Element> for Standard { //Релізація генератора рандомних значень для списку елементів
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

impl Distribution<Effect> for Standard { //Генератор рандомних значень для списку ефектів
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Effect {
        match rng.gen_range(0..72) {
            0..=59 => Effect::Atk(rng.gen_range(1..=12)), //~83%
            60..=63 => Effect::Flow, //~5,5%
            64..=67 => Effect::Stun, //~5,5%
            _ => Effect::Add(rng.gen_range(1..=4)), //~5,5%
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Card { //Структура, що описує карту
    element: Element,
    effect: Effect, 
}

impl Card { //Методи структури Card
    pub fn new(element: Element, effect: Effect) -> Self { //Конструктор
        Self {
            element,
            effect,
        }
    }

    pub fn play(&self, card: Self) -> Result<Effect, ()> { //Метод для битви двох карт
        let coef = self.element.coefficient(card.element); //визначення коефіцієнту
        let other_power = match card.effect { //Визначення базової атаки попередньої карти
            Effect::Atk(power) => power, //Якщо ефект Atk то це є значенням цього ефекту
            _ => 1, //інакше 1
        };
        match self.effect { //Обробка ефектів щойно зіграної карти
            Effect::Atk(power) => { //Якщо ефект Atk
                if (power as f32 *coef).round() < other_power as f32 { return Err(()) } //То порівняти базову атаку помножену на коефіцієнт з атакою попередньої карти
                //повернути помилку якщо менше
                Ok(Effect::Atk(power))
            },
            effect => { //Якщо будь-який інший ефект
                if coef < 1.0 { return Err(()) } //повернути помилку якщо коефіцієнт менший за 1
                Ok(effect)
            },
        }
        //при успіху функція повертає ефект
    }
}

impl Distribution<Card> for Standard { //Генератор рандомних значень для Card
    fn sample<R: Rng + ?Sized>(&self, _rng: &mut R) -> Card {
        Card {
            element: rand::random(),
            effect: rand::random(), 
        }

    }
}