// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::components::pin::{Mode, PinRef};

#[inline]
pub fn pins_to_value(pins: &Vec<PinRef>) -> usize {
    let mut value = 0;
    for (i, pin) in pins.into_iter().enumerate() {
        value |= (match level!(pin) {
            Some(v) if v >= 0.5 => 1,
            _ => 0,
        }) << i;
    }
    value
}

#[inline]
pub fn value_to_pins(value: usize, pins: &Vec<PinRef>) {
    for (i, pin) in pins.into_iter().enumerate() {
        set_level!(pin, Some(((value >> i) & 1) as f64));
    }
}

#[inline]
pub fn mode_to_pins(mode: Mode, pins: &Vec<PinRef>) {
    for pin in pins {
        set_mode!(pin, mode);
    }
}

#[inline]
pub fn value_high(value: Option<f64>) -> bool {
    match value {
        Some(v) if v >= 0.5 => true,
        _ => false,
    }
}
