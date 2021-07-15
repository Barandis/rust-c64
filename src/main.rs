// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

#[macro_use]
mod macros;

pub mod chips;
pub mod components;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

fn main() {
    println!("Hello, world!");
}
