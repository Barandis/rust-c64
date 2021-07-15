// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

// Note that the imports for std::rc::Rc, std::cell::RefCell, and
// crate::components::pin::Pin are only necessary because of the demo non-macro constructor
// function.

/// Pin assignment constants for the Ic7406 struct.
pub mod constants {
    /// The pin assignment for the input of inverter 1.
    pub const A1: usize = 1;
    /// The pin assignment for the input of inverter 2.
    pub const A2: usize = 3;
    /// The pin assignment for the input of inverter 3.
    pub const A3: usize = 5;
    /// The pin assignment for the input of inverter 4.
    pub const A4: usize = 9;
    /// The pin assignment for the input of inverter 5.
    pub const A5: usize = 11;
    /// The pin assignment for the input of inverter 6.
    pub const A6: usize = 13;

    /// The pin assignment for the output of inverter 1.
    pub const Y1: usize = 2;
    /// The pin assignment for the output of inverter 2.
    pub const Y2: usize = 4;
    /// The pin assignment for the output of inverter 3.
    pub const Y3: usize = 6;
    /// The pin assignment for the output of inverter 4.
    pub const Y4: usize = 8;
    /// The pin assignment for the output of inverter 5.
    pub const Y5: usize = 10;
    /// The pin assignment for the output of inverter 6.
    pub const Y6: usize = 12;

    /// The pin assignment for the +5V power supply.
    pub const VCC: usize = 14;
    /// The pin assignment for the ground.
    pub const GND: usize = 7;
}

use std::{cell::RefCell, rc::Rc};

use crate::{
    components::{
        device::{Device, DeviceRef, LevelChange, DUMMY},
        pin::{
            Mode::{Input, Output, Unconnected},
            Pin, PinRef,
        },
    },
    utils::value_high,
};

use self::constants::*;

const INPUTS: [usize; 6] = [A1, A2, A3, A4, A5, A6];

/// An emulation of the 7406 hex inverter.
///
/// The 7406 is one of the 7400-series TTL logic chips, consisting of six single-input
/// inverters. An inverter is the simplest of logic gates: if the input is low, the output
/// is high, and vice versa.
///
/// | An    | Yn    |
/// | :---: | :---: |
/// | L     | **H** |
/// | H     | **L** |
///
/// The chip comes in a 14-pin dual in-line package with the following pin assignments.
/// ```txt
///         +---+--+---+
///      A1 |1  +--+ 14| Vcc
///      Y1 |2       13| A6
///      A2 |3       12| Y6
///      Y2 |4  7406 11| A5
///      A3 |5       10| Y5
///      Y3 |6        9| A4
///     GND |7        8| Y4
///         +----------+
/// ```
/// GND and Vcc are ground and power supply pins respectively, and they are not emulated.
///
/// In the Commodore 64, U8 is a 7406. It's responsible for inverting logic signals that are
/// expected in the inverse they're given, such as the 6567's AEC signal being turned into
/// the inverse AEC signal for the 82S100.
pub struct Ic7406 {
    /// The pins of the 7406, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: Vec<PinRef>,
}

impl Ic7406 {
    /// Creates a new 7406 hex inverter emulation and returns a shared, internally mutable
    /// reference to it.
    pub fn new() -> DeviceRef {
        // Input pins. In the TI data sheet, these are named "1A", "2A", etc., and the C64
        // schematic does not suggest names for them. Since these names are not legal
        // variable names, we've switched the letter and number.
        let a1 = pin!(A1, "A1", Input);
        let a2 = pin!(A2, "A2", Input);
        let a3 = pin!(A3, "A3", Input);
        let a4 = pin!(A4, "A4", Input);
        let a5 = pin!(A5, "A5", Input);
        let a6 = pin!(A6, "A6", Input);

        // Output pins. Similarly, the TI data sheet refers to these as "1Y", "2Y", etc.
        let y1 = pin!(Y1, "Y1", Output);
        let y2 = pin!(Y2, "Y2", Output);
        let y3 = pin!(Y3, "Y3", Output);
        let y4 = pin!(Y4, "Y4", Output);
        let y5 = pin!(Y5, "Y5", Output);
        let y6 = pin!(Y6, "Y6", Output);

        // Power supply and ground pins, not emulated
        let gnd = pin!(GND, "GND", Unconnected);
        let vcc = pin!(VCC, "VCC", Unconnected);

        let chip: DeviceRef = new_ref!(Ic7406 {
            pins: pins![a1, a2, a3, a4, a5, a6, y1, y2, y3, y4, y5, y6, vcc, gnd],
        });

        // All outputs begin high since all of the inputs begin non-high.
        set!(y1, y2, y3, y4, y5, y6);

        attach!(a1, clone_ref!(chip));
        attach!(a2, clone_ref!(chip));
        attach!(a3, clone_ref!(chip));
        attach!(a4, clone_ref!(chip));
        attach!(a5, clone_ref!(chip));
        attach!(a6, clone_ref!(chip));

        chip
    }

    /// Creates a new Ic7406 hex inverter emulation and returns a shared, internally mutable
    /// reference to it. This is identical to `new` except that this one is coded without
    /// the benefit of crate-defined macros or type aliases (the vec! macro is still used,
    /// but that's standard library). It's here in this struct only for demonstration
    /// purposes.
    pub fn new_no_macro() -> Rc<RefCell<dyn Device>> {
        // Dummy pin, used as a spacer to put the index of the first real pin at 1.
        let dummy = Pin::new(0, DUMMY, Unconnected);

        // Input pins. In the TI data sheet, these are named "1A", "2A", etc., and the C64
        // schematic does not suggest names for them. Since these names are not legal
        // variable names, we've switched the letter and number.
        let a1 = Pin::new(A1, "A1", Input);
        let a2 = Pin::new(A2, "A2", Input);
        let a3 = Pin::new(A3, "A3", Input);
        let a4 = Pin::new(A4, "A4", Input);
        let a5 = Pin::new(A5, "A5", Input);
        let a6 = Pin::new(A6, "A6", Input);

        // Output pins. Similarly, the TI data sheet refers to these as "1Y", "2Y", etc.
        let y1 = Pin::new(Y1, "Y1", Output);
        let y2 = Pin::new(Y2, "Y2", Output);
        let y3 = Pin::new(Y3, "Y3", Output);
        let y4 = Pin::new(Y4, "Y4", Output);
        let y5 = Pin::new(Y5, "Y5", Output);
        let y6 = Pin::new(Y6, "Y6", Output);

        // Power supply and ground pins, not emulated
        let gnd = Pin::new(GND, "GND", Unconnected);
        let vcc = Pin::new(VCC, "VCC", Unconnected);

        let chip: Rc<RefCell<dyn Device>> = Rc::new(RefCell::new(Ic7406 {
            pins: vec![
                Rc::clone(&dummy),
                Rc::clone(&a1),
                Rc::clone(&y1),
                Rc::clone(&a2),
                Rc::clone(&y2),
                Rc::clone(&a3),
                Rc::clone(&y3),
                Rc::clone(&gnd),
                Rc::clone(&y4),
                Rc::clone(&a4),
                Rc::clone(&y5),
                Rc::clone(&a5),
                Rc::clone(&y6),
                Rc::clone(&a6),
                Rc::clone(&vcc),
            ],
        }));

        // All outputs begin high since all of the inputs begin non-high.
        y1.borrow_mut().set();
        y2.borrow_mut().set();
        y3.borrow_mut().set();
        y4.borrow_mut().set();
        y5.borrow_mut().set();
        y6.borrow_mut().set();

        a1.borrow_mut().attach(Rc::clone(&chip));
        a2.borrow_mut().attach(Rc::clone(&chip));
        a3.borrow_mut().attach(Rc::clone(&chip));
        a4.borrow_mut().attach(Rc::clone(&chip));
        a5.borrow_mut().attach(Rc::clone(&chip));
        a6.borrow_mut().attach(Rc::clone(&chip));

        chip
    }
}

/// Maps each input pin assignment ot its corresponding output pin assignment.
fn output_for(input: usize) -> usize {
    match input {
        A1 => Y1,
        A2 => Y2,
        A3 => Y3,
        A4 => Y4,
        A5 => Y5,
        A6 => Y6,
        _ => 0,
    }
}

impl Device for Ic7406 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        Vec::new()
    }

    fn update(&mut self, event: &LevelChange) {
        match event {
            LevelChange(pin, _, level) if INPUTS.contains(&number!(pin)) => {
                let o = output_for(number!(pin));
                if value_high(*level) {
                    clear!(self.pins[o]);
                } else {
                    set!(self.pins[o]);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{components::trace::TraceRef, test_utils::make_traces};

    use super::*;

    fn before_each() -> (DeviceRef, Vec<TraceRef>) {
        let chip = Ic7406::new();
        let tr = make_traces(clone_ref!(chip));
        (chip, tr)
    }

    #[test]
    fn input_high() {
        let (_, tr) = before_each();

        set!(tr[A1]);
        assert!(low!(tr[Y1]), "Y1 should be low when A1 is high");

        set!(tr[A2]);
        assert!(low!(tr[Y2]), "Y2 should be low when A2 is high");

        set!(tr[A3]);
        assert!(low!(tr[Y3]), "Y3 should be low when A3 is high");

        set!(tr[A4]);
        assert!(low!(tr[Y4]), "Y4 should be low when A4 is high");

        set!(tr[A5]);
        assert!(low!(tr[Y5]), "Y5 should be low when A5 is high");

        set!(tr[A6]);
        assert!(low!(tr[Y6]), "Y6 should be low when A6 is high");
    }

    #[test]
    fn input_low() {
        let (_, tr) = before_each();

        clear!(tr[A1]);
        assert!(high!(tr[Y1]), "Y1 should be high when A1 is low");

        clear!(tr[A2]);
        assert!(high!(tr[Y2]), "Y2 should be high when A2 is low");

        clear!(tr[A3]);
        assert!(high!(tr[Y3]), "Y3 should be high when A3 is low");

        clear!(tr[A4]);
        assert!(high!(tr[Y4]), "Y4 should be high when A4 is low");

        clear!(tr[A5]);
        assert!(high!(tr[Y5]), "Y5 should be high when A5 is low");

        clear!(tr[A6]);
        assert!(high!(tr[Y6]), "Y6 should be high when A6 is low");
    }

    // Duplicate tests using no macros. These use the non-macro creation function as well
    // because I like the symmetry. Only this struct has non-macro versions of the tests,
    // and it's just for demonstration purposes.

    #[test]
    fn input_high_no_macro() {
        let (_, tr) = before_each();

        tr[A1].borrow_mut().set();
        assert!(tr[Y1].borrow().low(), "Y1 should be low when A1 is high");

        tr[A2].borrow_mut().set();
        assert!(tr[Y2].borrow().low(), "Y2 should be low when A2 is high");

        tr[A3].borrow_mut().set();
        assert!(tr[Y3].borrow().low(), "Y3 should be low when A3 is high");

        tr[A4].borrow_mut().set();
        assert!(tr[Y4].borrow().low(), "Y4 should be low when A4 is high");

        tr[A5].borrow_mut().set();
        assert!(tr[Y5].borrow().low(), "Y5 should be low when A5 is high");

        tr[A6].borrow_mut().set();
        assert!(tr[Y6].borrow().low(), "Y6 should be low when A6 is high");
    }

    #[test]
    fn input_low_no_macro() {
        let (_, tr) = before_each();

        tr[A1].borrow_mut().clear();
        assert!(tr[Y1].borrow().high(), "Y1 should be high when A1 is low");

        tr[A2].borrow_mut().clear();
        assert!(tr[Y2].borrow().high(), "Y2 should be high when A2 is low");

        tr[A3].borrow_mut().clear();
        assert!(tr[Y3].borrow().high(), "Y3 should be high when A3 is low");

        tr[A4].borrow_mut().clear();
        assert!(tr[Y4].borrow().high(), "Y4 should be high when A4 is low");

        tr[A5].borrow_mut().clear();
        assert!(tr[Y5].borrow().high(), "Y5 should be high when A5 is low");

        tr[A6].borrow_mut().clear();
        assert!(tr[Y6].borrow().high(), "Y6 should be high when A6 is low");
    }
}
