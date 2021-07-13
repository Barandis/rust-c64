// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// The pin assignment for the first input of gate 1.
    pub const A1: usize = 1;
    /// The pin assignment for the second input of gate 1.
    pub const B1: usize = 2;
    /// The pin assignment for the output of gate 1.
    pub const Y1: usize = 3;

    /// The pin assignment for the first input of gate 2.
    pub const A2: usize = 4;
    /// The pin assignment for the second input of gate 2.
    pub const B2: usize = 5;
    /// The pin assignment for the output of gate 2.
    pub const Y2: usize = 6;

    /// The pin assignment for the first input of gate 3.
    pub const A3: usize = 9;
    /// The pin assignment for the second input of gate 3.
    pub const B3: usize = 10;
    /// The pin assignment for the output of gate 3.
    pub const Y3: usize = 8;

    /// The pin assignment for the first input of gate 4.
    pub const A4: usize = 12;
    /// The pin assignment for the second input of gate 4.
    pub const B4: usize = 13;
    /// The pin assignment for the output of gate 4.
    pub const Y4: usize = 11;

    /// The pin assignment for the +5V power supply.
    pub const VCC: usize = 14;
    /// The pin assignment for the ground.
    pub const GND: usize = 7;
}

use crate::components::{
    device::{Device, DeviceRef, LevelChangeEvent},
    pin::{
        Mode::{Input, Output, Unconnected},
        PinRef,
    },
};

use self::constants::*;

const INPUTS: [usize; 8] = [A1, A2, A3, A4, B1, B2, B3, B4];

/// An emulation of the 7408 quad two-input AND gate.
///
/// The 7408 is one of the 7400-series TTL logic circuits, consisting of four dual-input AND
/// gates. An AND gate's output is high as long as all of its outputs are high; otherwise
/// the output is low.
///
/// The A and B pins are inputs while the Y pins are the outputs.
///
/// | An    | Bn    | Yn    |
/// | :---: | :---: | :---: |
/// | L     | L     | **L** |
/// | L     | H     | **L** |
/// | H     | L     | **L** |
/// | H     | H     | **H** |
///
/// The chip comes in a 14-pin dual in-line package with the following pin assignments.
/// ```text
///         +---+--+---+
///      A1 |1  +--+ 14| Vcc
///      B1 |2       13| B4
///      Y1 |3       12| A4
///      A2 |4  7408 11| Y4
///      B2 |5       10| B3
///      Y2 |6        9| A3
///     GND |7        8| Y3
///         +----------+
/// ```
/// GND and Vcc are ground and power supply pins respectively, and they are not emulated.
///
/// In the Commodore 64, U27 is a 74LS08 (a lower-power, faster variant whose emulation is
/// the same). It's used for combining control signals from various sources, such as the BA
/// signal from the 6567 VIC and the DMA signal from the expansion port combining into the
/// `RDY` signal for the 6510 CPU.
pub struct Ic7408 {
    /// The pins of the 7408, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: Vec<PinRef>,
}

impl Ic7408 {
    /// Creates a new 7408 quad 2-input AND gate emulation and returns a shared, internally
    /// mutable reference to it.
    pub fn new() -> DeviceRef {
        // Gate 1 inputs and output
        let a1 = pin!(A1, "A1", Input);
        let b1 = pin!(B1, "B1", Input);
        let y1 = pin!(Y1, "Y1", Output);

        // Gate 2 inputs and output
        let a2 = pin!(A2, "A2", Input);
        let b2 = pin!(B2, "B2", Input);
        let y2 = pin!(Y2, "Y2", Output);

        // Gate 3 inputs and output
        let a3 = pin!(A3, "A3", Input);
        let b3 = pin!(B3, "B3", Input);
        let y3 = pin!(Y3, "Y3", Output);

        // Gate 4 inputs and output
        let a4 = pin!(A4, "A4", Input);
        let b4 = pin!(B4, "B4", Input);
        let y4 = pin!(Y4, "Y4", Output);

        // Power supply and ground pins, not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let chip: DeviceRef = new_ref!(Ic7408 {
            pins: pins![a1, a2, a3, a4, b1, b2, b3, b4, y1, y2, y3, y4, vcc, gnd],
        });

        // All output pins begin low because none have any high inputs.
        clear!(y1, y2, y3, y4);

        attach!(a1, clone_ref!(chip));
        attach!(b1, clone_ref!(chip));
        attach!(a2, clone_ref!(chip));
        attach!(b2, clone_ref!(chip));
        attach!(a3, clone_ref!(chip));
        attach!(b3, clone_ref!(chip));
        attach!(a4, clone_ref!(chip));
        attach!(b4, clone_ref!(chip));

        chip
    }
}

/// Maps each input pin assignment to a tuple of its gate's other input pin assignment and
/// its gate's output pin assignment.
fn input_output_for(input: usize) -> (usize, usize) {
    match input {
        A1 => (B1, Y1),
        B1 => (A1, Y1),
        A2 => (B2, Y2),
        B2 => (A2, Y2),
        A3 => (B3, Y3),
        B3 => (A3, Y3),
        A4 => (B4, Y4),
        B4 => (A4, Y4),
        _ => (0, 0),
    }
}

impl Device for Ic7408 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChangeEvent) {
        match event {
            LevelChangeEvent(p, _, level) if INPUTS.contains(p) => match level {
                Some(value) if *value >= 0.5 => {
                    let (i, o) = input_output_for(*p);
                    if high!(self.pins[i]) {
                        set!(self.pins[o]);
                    } else {
                        clear!(self.pins[o]);
                    }
                }
                _ => {
                    let (_, o) = input_output_for(*p);
                    clear!(self.pins[o]);
                }
            },
            _ => (),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test_utils::make_traces;

    use super::*;

    #[test]
    fn gate_1() {
        let chip = Ic7408::new();
        let tr = make_traces(&chip);

        clear!(tr[A1]);
        clear!(tr[B1]);
        assert!(low!(tr[Y1]), "Y1 should be low when A1 and B1 are both low");

        clear!(tr[A1]);
        set!(tr[B1]);
        assert!(
            low!(tr[Y1]),
            "Y1 should be low when A1 is low and B1 is high"
        );

        set!(tr[A1]);
        clear!(tr[B1]);
        assert!(
            low!(tr[Y1]),
            "Y1 should be low when A1 is high and B1 is low"
        );

        set!(tr[A1]);
        set!(tr[B1]);
        assert!(
            high!(tr[Y1]),
            "Y1 should be high when A1 and B1 are both high"
        );
    }

    #[test]
    fn gate_2() {
        let chip = Ic7408::new();
        let tr = make_traces(&chip);

        clear!(tr[A2]);
        clear!(tr[B2]);
        assert!(low!(tr[Y2]), "Y2 should be low when A2 and B2 are both low");

        clear!(tr[A2]);
        set!(tr[B2]);
        assert!(
            low!(tr[Y2]),
            "Y2 should be low when A2 is low and B2 is high"
        );

        set!(tr[A2]);
        clear!(tr[B2]);
        assert!(
            low!(tr[Y2]),
            "Y2 should be low when A2 is high and B2 is low"
        );

        set!(tr[A2]);
        set!(tr[B2]);
        assert!(
            high!(tr[Y2]),
            "Y2 should be high when A2 and B2 are both high"
        );
    }

    #[test]
    fn gate_3() {
        let chip = Ic7408::new();
        let tr = make_traces(&chip);

        clear!(tr[A3]);
        clear!(tr[B3]);
        assert!(low!(tr[Y3]), "Y3 should be low when A3 and B3 are both low");

        clear!(tr[A3]);
        set!(tr[B3]);
        assert!(
            low!(tr[Y3]),
            "Y3 should be low when A3 is low and B3 is high"
        );

        set!(tr[A3]);
        clear!(tr[B3]);
        assert!(
            low!(tr[Y3]),
            "Y3 should be low when A3 is high and B3 is low"
        );

        set!(tr[A3]);
        set!(tr[B3]);
        assert!(
            high!(tr[Y3]),
            "Y3 should be high when A3 and B3 are both high"
        );
    }

    #[test]
    fn gate_4() {
        let chip = Ic7408::new();
        let tr = make_traces(&chip);

        clear!(tr[A4]);
        clear!(tr[B4]);
        assert!(low!(tr[Y4]), "Y4 should be low when A4 and B4 are both low");

        clear!(tr[A4]);
        set!(tr[B4]);
        assert!(
            low!(tr[Y4]),
            "Y4 should be low when A4 is low and B4 is high"
        );

        set!(tr[A4]);
        clear!(tr[B4]);
        assert!(
            low!(tr[Y4]),
            "Y4 should be low when A4 is high and B4 is low"
        );

        set!(tr[A4]);
        set!(tr[B4]);
        assert!(
            high!(tr[Y4]),
            "Y4 should be high when A4 and B4 are both high"
        );
    }
}
