// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{
    cell::RefCell,
    fmt::{Debug, Formatter, Result},
    rc::Rc,
};

use crate::components::pin::Mode::{Bidirectional, Input, Output, Unconnected};

use super::pin::PinRef;

pub type DeviceRef = Rc<RefCell<dyn Device>>;

pub const DUMMY: &str = "__DUMMY__";
pub trait Device {
    fn pins(&self) -> Vec<PinRef>;
    fn registers(&self) -> Vec<u8>;
    fn update(&mut self, event: &LevelChangeEvent);

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

            for pin in self.pins() {
                if name!(pin) != DUMMY {
                    str.push_str("(number = ");
                    str.push_str(format!("{}", number!(pin)).as_str());
                    str.push_str(", name = ");
                    str.push_str(name!(pin));
                    str.push_str(", mode = ");
                    str.push_str(match mode!(pin) {
                        Unconnected => "Unconnected",
                        Input => "Input",
                        Output => "Output",
                        Bidirectional => "Bidirectional",
                    });
                    str.push_str(", level = ");
                    str.push_str(format!("{:?}", level!(pin)).as_str());
                    str.push(')');
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct LevelChangeEvent(pub usize, pub Option<f64>, pub Option<f64>);
