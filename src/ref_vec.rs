// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::ops::{Deref, DerefMut};
use std::{cell::RefCell, rc::Rc};

pub struct RefVec<T>(Vec<Rc<RefCell<T>>>);

pub struct RefIter<'a, T>(&'a [Rc<RefCell<T>>]);

impl<T> RefVec<T> {
    pub const fn new() -> RefVec<T> {
        RefVec(Vec::new())
    }

    pub const fn with_vec(v: Vec<Rc<RefCell<T>>>) -> RefVec<T> {
        RefVec(v)
    }

    pub fn get_ref(&self, index: usize) -> Rc<RefCell<T>> {
        Rc::clone(&self[index])
    }

    pub fn iter_ref(&self) -> RefIter<'_, T> {
        RefIter(self.0.as_slice())
    }

    pub fn as_refs(&self) -> RefVec<T> {
        RefVec(
            self.0
                .iter()
                .map(|pin| Rc::clone(pin))
                .collect::<Vec<Rc<RefCell<T>>>>(),
        )
    }
}

impl<'a, T> Iterator for RefIter<'a, T> {
    type Item = Rc<RefCell<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.get(0) {
            Some(item) => {
                self.0 = &self.0[1..];
                Some(Rc::clone(item))
            }
            None => None,
        }
    }
}

impl<T> Deref for RefVec<T> {
    type Target = Vec<Rc<RefCell<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RefVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
