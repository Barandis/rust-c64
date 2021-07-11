// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{cell::RefCell, rc::Rc};

use super::pin::PinRef;

pub type DeviceRef = Rc<RefCell<dyn Device>>;
pub trait Device {
    fn pins(&self) -> Vec<PinRef>;
    fn registers(&self) -> Vec<u8>;
    fn update(&mut self, event: &LevelChangeEvent);
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LevelChangeEvent(pub usize, pub Option<f64>, pub Option<f64>);
