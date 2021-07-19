// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

#[macro_use]
mod macros;

pub mod components;
pub mod devices;
pub mod roms;
pub mod utils;
pub mod vectors;

#[cfg(test)]
pub mod test_utils;

fn main() {
    println!("Hello, world!");
}
