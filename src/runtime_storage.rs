use serde::Serialize;
use crate::gateway::events::{SharedTableEvents, TableEvents};
use std::{
    borrow::Borrow,
    collections::HashSet,
    ops::Deref,
    hash::Hash,
};

#[derive(Debug, Serialize, Clone)]
pub struct DataTable<T>(pub HashSet<T>);

impl<T> DataTable<T>
where T: Eq + PartialEq + Hash + Clone {
    pub fn new() -> Self {
        Self(HashSet::<T>::new())
    }
}

pub trait Table<T: Eq + PartialEq + Hash + TableEvents + Clone> {
    fn insert(&mut self, value: T) -> bool; 
    fn replace(&mut self, value: T) -> Option<T>;
    fn update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E>
    where
        Q: Hash + Eq,
        T: Borrow<Q>, 
        F: FnOnce(&mut T) -> Result<(), E>;
    fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Eq,
        T: Borrow<Q>;
    
}

impl<T> Table<T> for DataTable<T>
where T: Eq + PartialEq + Hash + TableEvents + Clone {
    fn insert(&mut self, value: T) -> bool {
        let result = self.0.insert(value.clone());
        if result {
            value.insert();
        }
        result
    }

    fn replace(&mut self, value: T) -> Option<T> {
        let record = self.0.replace(value.clone());
        if record.is_some() {
            value.update()
        }
        else {
            value.insert()
        }
        record
    }

    fn update<F, E, Q>(&mut self, value: &Q, func: F) -> Result<Option<T>, E>
    where
        Q: Hash + Eq,
        T: Borrow<Q>, 
        F: FnOnce(&mut T) -> Result<(), E>
    {
        let Some(original) = self.get(&value) else { return Ok(None) };
        let mut record = original.clone();
        func(&mut record)?;
        Ok(self.replace(record))
    }

    fn remove<Q>(&mut self, value: &Q) -> bool
    where
        Q: Hash + Eq,
        T: Borrow<Q>,
    {
        if let Some(record) = self.0.take::<Q>(value) {
            record.delete();
            true
        } else {
            false
        }
    }
}

pub trait SharedTable<T: Eq + PartialEq + Hash + SharedTableEvents + Clone> {
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
            for i in self.0.iter() {
                i.insert(value.clone())
            }
        }
        result
    }

    fn shared_replace(&mut self, value: T) -> Option<T> {
        let record = self.0.replace(value.clone());
        for i in self.0.iter() {
            if record.is_some() {
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

impl<T> Deref for DataTable<T> {
    type Target = HashSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}