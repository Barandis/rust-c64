// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{
    cell::RefCell,
    fmt::{Debug, Formatter, Result},
    rc::Rc,
};

use crate::{
    components::pin::{
        Mode::{Bidirectional, Input, Output, Unconnected},
        Pin,
    },
    ref_vec::RefVec,
};

pub type DeviceRef = Rc<RefCell<dyn Device>>;

pub const DUMMY: &str = "__DUMMY__";

pub trait Device {
    // I would like to use an array here instead of a Vec - the array is set at creation
    // time and never changes, so the mutability of a Vec is not necessary. Unfortunately,
    // const generics are necessary to do this, and while they now exist, they do not allow
    // inference (i.e., if you have a Pin that has a reference to a Device<const P: usize,
    // const R: usize>, you can't have that reference just be a DeviceRef<_, _> even though
    // you, as a Pin, don't care about the size of the pin and register arrays).
    //
    // This means that since Pin has a Device, Pin would have to also have two const
    // generics, which means that since Trace holds a list of Pins, Trace would also have to
    // have two const generics...and since we can't infer const generics yet, that's a
    // problem. Each of Trace's Pins can come from different Devices, meaning they'd have
    // different const generics, and we can't yet express that.
    fn pins(&self) -> RefVec<Pin>;
    // Also would like to use an array here, but same const generic problem.
    fn registers(&self) -> Vec<u8>;
    fn update(&mut self, event: &LevelChange);

    fn debug_fmt(&self, f: &mut Formatter) -> Result {
        let alt = f.alternate();
        let mut str = String::from("Device {");
        let len = self.pins().len();

        if len > 0 {
            if alt {
                str.push_str("\n    pins: [\n        ");
            } else {
                str.push_str(" pins: [");
            }

            for pin in self.pins().iter() {
                if name!(pin) != DUMMY {
                    str.push_str(
                        format!(
                            "[{0:>1$}] {2:3$} ({4}): {5}",
                            number!(pin),
                            if alt { 2 } else { 1 },
                            name!(pin),
                            if alt { 6 } else { 1 },
                            match mode!(pin) {
                                Unconnected => "U",
                                Input => "I",
                                Output => "O",
                                Bidirectional => "B",
                            },
                            match level!(pin) {
                                Some(v) =>
                                    if v == 0.0 {
                                        String::from("0")
                                    } else if v == 1.0 {
                                        String::from("1")
                                    } else {
                                        format!("{}", v)
                                    },
                                None => String::from("-"),
                            }
                        )
                        .as_str(),
                    );
                    if number!(pin) < len - 1 {
                        if alt {
                            str.push_str(",\n        ");
                        } else {
                            str.push_str(", ");
                        }
                    }
                }
            }

            if alt {
                str.push_str("\n    ]\n");
            } else {
                str.push(']');
            }
        }
        if alt {
            str.push('}');
        } else {
            str.push_str(" }");
        }

        write!(f, "{}", str)
    }
}

impl Debug for dyn Device {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.debug_fmt(f)
    }
}

#[derive(Clone, Debug)]
pub struct LevelChange<'a>(pub Rc<RefCell<&'a Pin>>);
