use serde::Serialize;
use crate::gateway::events::{SharedTableEvents, TableEvents};
use std::{
    borrow::Borrow,
    collections::HashSet,
    ops::Deref,
    hash::Hash,
};

#[derive(Debug, Serialize, Clone)]
pub struct DataTable<T>(pub HashSet<T>); //Нова структура даних, що обертає структуру HashSet

impl<T> DataTable<T>
where T: Eq + PartialEq + Hash + Clone { //Задання методів
    pub fn new() -> Self {
        Self(HashSet::<T>::new())
    }
}

pub trait Table<T: Eq + PartialEq + Hash + TableEvents + Clone> { //Створення ознаки Table<T>, події про зміну відправляються тільки обє'ктам, що змінюються
    /*об'єкт T повинен бути порівнювальним,
    мати реалізацію ознаки Hash для перетворення себе у хеш-значення,
    реалізацію ознаки TableEvents та можливість бути клонованим*/
    fn insert(&mut self, value: T) -> bool; 
    fn replace(&mut self, value: T) -> Option<T>;
    fn update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E> //Q - значення-запит,
    where
        Q: Hash + Eq, //Q повинен бути порівнювальним та мати ознаку Hash
        T: Borrow<Q>, //для цього методу T повинен мати ознаку яка вказує поле типу Q на яке повинен бути створений показник,
        F: FnOnce(&mut T) -> Result<(), E>; //F - функція яка виконується лише раз - { } з кодом всередені переданий як аргумент
        /*це закриття матиме доступ до об'єкту T та повинне повертати результат своєї роботи
        Приклад:
            self.update("value", |t| {
                t = "updated value";
                println!("New value: {}", t);
                Ok(())
            });
        */
    fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Eq,
        T: Borrow<Q>;
    
}

impl<T> Table<T> for DataTable<T>
where T: Eq + PartialEq + Hash + TableEvents + Clone { //Реалізація ознаки Table для DataTable
    fn insert(&mut self, value: T) -> bool {
        let result = self.0.insert(value.clone()); //При успішному вставлені в набір значень повернеться true
        if result {
            value.insert(); //Виклик методу insert з ознаки TableEvents
        }
        result
    }

    fn replace(&mut self, value: T) -> Option<T> {
        let record = self.0.replace(value.clone()); //Заміна повертає значення до цієї операції
        if record.is_some() {
            value.update() //Якщо значення вже було в наборі, тоді надіслати подію про оновлення
        }
        else {
            value.insert() //інакше подію про вставлення в набір
        }
        record
    }

    fn update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E>
    where
        Q: Hash + Eq,
        T: Borrow<Q>, 
        F: FnOnce(&mut T) -> Result<(), E>
    {
        let Some(original) = self.get(&value) else { return Ok(None) }; //Дістати значення з набору, якщо такого значення немає значить і оновлювати нічого
        let mut record = original.clone(); //клонувати значення з набору у нову змінну з можливістю зміни значень.
        func(&mut record)?; //Виконати отримане закриття
        Ok(self.replace(record)) //Замінити значення на оновлене в функції F методом цієї ознаки
    }

    fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Eq,
        T: Borrow<Q>,
    {
        if let Some(record) = self.0.take::<Q>(value) { //Забрати значення з набору
            record.delete(); //Надіслати подію про видалення якщо значення існувало
            true
        } else {
            false
        }
    }
}

pub trait SharedTable<T: Eq + PartialEq + Hash + SharedTableEvents + Clone> { //Створення ознаки SharedTable<T>, події про зміну відправляються усім об'єктам в наборі
    //обмеження відповідають ознаці Table, з відмінністю у вимозі до наявності ознаки подій у T, для цієї ознаки T повинен мати реалізацію ознаки SharedTableEvents
    fn shared_insert(&mut self, value: T) -> bool; 
    fn shared_replace(&mut self, value: T) -> Option<T>;
    fn shared_update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E>
    where
        Q: Hash + Eq,
        T: Borrow<Q>, 
        F: FnOnce(&mut T) -> Result<(), E>;
    fn shared_remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Eq,
        T: Borrow<Q>;
    
}
impl<T> SharedTable<T> for DataTable<T>
where T: Eq + PartialEq + Hash + SharedTableEvents + Clone {
    fn shared_insert(&mut self, value: T) -> bool {
        let result = self.0.insert(value.clone());
        if result {
            for i in self.0.iter() { //обхід усих об'єктів в наборі
                i.insert(value.clone()) //та надсилання їм події про нове значення
            }
        }
        result
    }

    fn shared_replace(&mut self, value: T) -> Option<T> {
        let record = self.0.replace(value.clone());
        for i in self.0.iter() { //обхід
            if record.is_some() { //реалізація надсилання подій відповідає реалізації ознаки Table
                i.update(value.clone()) 
            }
            else {
                i.insert(value.clone())
            }
        } 
        record
    }

    fn shared_update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E>
        where
            Q: Hash + Eq,
            T: Borrow<Q>, 
            F: FnOnce(&mut T) -> Result<(), E> {
        let Some(original) = self.get(&value) else { return Ok(None) };
        let mut record = original.clone();
        func(&mut record)?;
        Ok(self.shared_replace(record))
    }
    
    fn shared_remove<Q>(&mut self, value: &Q) -> bool
        where
            Q: Hash + Eq,
            T: Borrow<Q> {
        if let Some(record) = self.0.take::<Q>(value) {
            for i in self.0.iter() {
                i.delete(record.clone())
            } 
            true
        } else {
            false
        }
    }
}

impl<T> Deref for DataTable<T> { //При звернені до об'єкту DataTable повертати посилання на HashSet, навколо якого обертається DataTable, для прямого доступу до значень
    type Target = HashSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}