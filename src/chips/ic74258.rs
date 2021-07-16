// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// The pin assignment for the select pin.
    pub const SEL: usize = 1;

    /// The pin assignment for the output enable pin.
    pub const OE: usize = 15;

    /// The pin assignment for the first input pin of multiplexer 1.
    pub const A1: usize = 2;
    /// The pin assignment for the second input pin of multiplexer 1.
    pub const B1: usize = 3;
    /// The pin assighment for the output pin of multiplexer 1.
    pub const Y1: usize = 4;

    /// The pin assignment for the first input pin of multiplexer 2.
    pub const A2: usize = 5;
    /// The pin assignment for the second input pin of multiplexer 2.
    pub const B2: usize = 6;
    /// The pin assighment for the output pin of multiplexer 2.
    pub const Y2: usize = 7;

    /// The pin assignment for the first input pin of multiplexer 3.
    pub const A3: usize = 11;
    /// The pin assignment for the second input pin of multiplexer 3.
    pub const B3: usize = 10;
    /// The pin assighment for the output pin of multiplexer 3.
    pub const Y3: usize = 9;

    /// The pin assignment for the first input pin of multiplexer 4.
    pub const A4: usize = 14;
    /// The pin assignment for the second input pin of multiplexer 4.
    pub const B4: usize = 13;
    /// The pin assighment for the output pin of multiplexer 4.
    pub const Y4: usize = 12;

    /// The pin assignment for the +5V power supply.
    pub const VCC: usize = 16;
    /// The pin assignment for the ground.
    pub const GND: usize = 8;
}

use crate::{
    components::{
        device::{Device, DeviceRef, LevelChange},
        pin::{
            Mode::{Input, Output, Unconnected},
            Pin, PinRef,
        },
    },
    ref_vec::RefVec,
    utils::value_high,
};

use self::constants::*;

const A_INPUTS: [usize; 4] = [A1, A2, A3, A4];
const B_INPUTS: [usize; 4] = [B1, B2, B3, B4];

/// An emulation of the 74258 quad 2-to-1 multiplexer.
///
/// The 74258 is one of the 7400-series TTL logic chips, consisting of four 2-to-1
/// multiplexers. Each multiplexer is essentially a switch which uses a single, shared
/// select signal to choose which of its two inputs to reflect on its output. Each output is
/// tri-state.
///
/// This chip is exactly the same as the 74257 except that this one has inverted outputs and
/// this other doesn't.
///
/// The inputs to each multiplexer are the A and B pins, and the Y pins are their outputs.
/// The SEL pin selects between the A inputs (when SEL is low) and the B inputs (when SEL is
/// high). This single pin selects the outputs for all four multiplexers simultaneously. The
/// active low output-enable pin, OE, tri-states all four outputs when it's set high.
///
/// | OE    | SEL   | An    | Bn    | Yn    |
/// | :---: | :---: | :---: | :---: | :---: |
/// | H     | X     | X     | X     | **Z** |
/// | L     | L     | L     | X     | **H** |
/// | L     | L     | H     | X     | **L** |
/// | L     | H     | X     | L     | **H** |
/// | L     | H     | X     | H     | **L** |
///
/// The chip comes in a 16-pin dual in-line package with the following pin assignments.
/// ```text
///         +---+--+---+
///     SEL |1  +--+ 16| VCC
///      A1 |2       15| OE
///      B1 |3       14| A4
///      Y1 |4       13| B4
///      A2 |5 74258 12| Y4
///      B2 |6       11| A3
///      Y2 |7       10| B3
///     GND |8        9| Y3
///         +----------+
/// ```
/// GND and VCC are ground and power supply pins respectively, and they are not emulated.
///
/// In the Commodore 64, U14 is a 74LS258 (a lower-power, faster variant whose emulation is
/// the same). It's used to multiplex the upper two lines of the multiplexed address bus
/// from the A6 and A7 lines from the 6567 VIC and the VA14 and VA15 lines from one of the
/// 6526 CIAs.
pub struct Ic74258 {
    /// The pins of the 74258, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,
}

impl Ic74258 {
    /// Creates a new 74258 quad 2-to-1 multiplexer emulation and returns a shared,
    /// internally mutable reference to it.
    pub fn new() -> DeviceRef {
        // Select. When this is low, the Y output pins will take on the inverse of the value
        // of their A input pins. When this is high, the Y output pins will instead take on
        // the inverse of the value of their B input pins.
        let sel = pin!(SEL, "SEL", Input);

        // Output enable. When this is high, all of the Y output pins will be forced into
        // hi-z, whatever the values of their input pins.
        let oe = pin!(OE, "OE", Input);

        // Multiplexer 1 inputs and output.
        let a1 = pin!(A1, "A1", Input);
        let b1 = pin!(B1, "B1", Input);
        let y1 = pin!(Y1, "Y1", Output);

        // Multiplexer 2 inputs and output.
        let a2 = pin!(A2, "A2", Input);
        let b2 = pin!(B2, "B2", Input);
        let y2 = pin!(Y2, "Y2", Output);

        // Multiplexer 3 inputs and output.
        let a3 = pin!(A3, "A3", Input);
        let b3 = pin!(B3, "B3", Input);
        let y3 = pin!(Y3, "Y3", Output);

        // Multiplexer 4 inputs and output.
        let a4 = pin!(A4, "A4", Input);
        let b4 = pin!(B4, "B4", Input);
        let y4 = pin!(Y4, "Y4", Output);

        // Power supply and ground pins, not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let device: DeviceRef = new_ref!(Ic74258 {
            pins: pins![a1, a2, a3, a4, b1, b2, b3, b4, y1, y2, y3, y4, oe, sel, vcc, gnd],
        });

        clear!(y1, y2, y3, y4);
        attach_to!(device, a1, a2, a3, a4, b1, b2, b3, b4, sel, oe);

        device
    }
}

/// Maps an input pin assignment to its corresponding output pin assignment.
fn output_for(input: usize) -> usize {
    match input {
        A1 | B1 => Y1,
        A2 | B2 => Y2,
        A3 | B3 => Y3,
        A4 | B4 => Y4,
        _ => 0,
    }
}

impl Device for Ic74258 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        macro_rules! select_a {
            () => {
                if high!(self.pins[A1]) {
                    clear!(self.pins[Y1]);
                } else {
                    set!(self.pins[Y1]);
                }
                if high!(self.pins[A2]) {
                    clear!(self.pins[Y2]);
                } else {
                    set!(self.pins[Y2]);
                }
                if high!(self.pins[A3]) {
                    clear!(self.pins[Y3]);
                } else {
                    set!(self.pins[Y3]);
                }
                if high!(self.pins[A4]) {
                    clear!(self.pins[Y4]);
                } else {
                    set!(self.pins[Y4]);
                }
            };
        }
        macro_rules! select_b {
            () => {
                if high!(self.pins[B1]) {
                    clear!(self.pins[Y1]);
                } else {
                    set!(self.pins[Y1]);
                }
                if high!(self.pins[B2]) {
                    clear!(self.pins[Y2]);
                } else {
                    set!(self.pins[Y2]);
                }
                if high!(self.pins[B3]) {
                    clear!(self.pins[Y3]);
                } else {
                    set!(self.pins[Y3]);
                }
                if high!(self.pins[B4]) {
                    clear!(self.pins[Y4]);
                } else {
                    set!(self.pins[Y4]);
                }
            };
        }

        match event {
            LevelChange(pin, _, level) if A_INPUTS.contains(&number!(pin)) => {
                let y = output_for(number!(pin));
                if high!(self.pins[OE]) {
                    float!(self.pins[y]);
                } else if low!(self.pins[SEL]) {
                    if value_high(*level) {
                        clear!(self.pins[y]);
                    } else {
                        set!(self.pins[y]);
                    }
                }
            }
            LevelChange(pin, _, level) if B_INPUTS.contains(&number!(pin)) => {
                let y = output_for(number!(pin));
                if high!(self.pins[OE]) {
                    float!(self.pins[y]);
                } else if high!(self.pins[SEL]) {
                    if value_high(*level) {
                        clear!(self.pins[y]);
                    } else {
                        set!(self.pins[y]);
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == SEL => {
                if high!(self.pins[OE]) {
                    float!(self.pins[Y1], self.pins[Y2], self.pins[Y3], self.pins[Y4]);
                } else {
                    if value_high(*level) {
                        select_b!();
                    } else {
                        select_a!();
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == OE => {
                if value_high(*level) {
                    float!(self.pins[Y1], self.pins[Y2], self.pins[Y3], self.pins[Y4]);
                } else {
                    if high!(self.pins[SEL]) {
                        select_b!();
                    } else {
                        select_a!();
                    }
                }
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{components::trace::Trace, test_utils::make_traces};

    use super::*;

    fn before_each() -> (DeviceRef, RefVec<Trace>) {
        let chip = Ic74258::new();
        let tr = make_traces(&chip);
        (chip, tr)
    }

    fn before_mux_1() -> (DeviceRef, RefVec<Trace>) {
        let (chip, tr) = before_each();
        clear!(tr[A1]);
        set!(tr[B1]);
        (chip, tr)
    }

    #[test]
    fn mux_1_select_a() {
        let (_, tr) = before_mux_1();

        clear!(tr[SEL]);
        assert!(
            high!(tr[Y1]),
            "Y1 should be high when A1 is low and SEL is low"
        );

        set!(tr[A1]);
        assert!(
            low!(tr[Y1]),
            "Y1 should be low when A1 is high and SEL is low"
        )
    }

    #[test]
    fn mux_1_select_b() {
        let (_, tr) = before_mux_1();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y1]),
            "Y1 should be low when B1 is high and SEL is high"
        );

        clear!(tr[B1]);
        assert!(
            high!(tr[Y1]),
            "Y1 should be high when B1 is low and SEL is high"
        );
    }

    #[test]
    fn mux_1_oe_high() {
        let (_, tr) = before_mux_1();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y1]),
            "Y1 should be low when B1 is high and SEL is high"
        );

        set!(tr[OE]);
        assert!(floating!(tr[Y1]), "Y1 should float when OE is high");

        clear!(tr[SEL]);
        assert!(floating!(tr[Y1]), "Y1 should float when OE is high");
    }

    fn before_mux_2() -> (DeviceRef, RefVec<Trace>) {
        let (chip, tr) = before_each();
        clear!(tr[A2]);
        set!(tr[B2]);
        (chip, tr)
    }

    #[test]
    fn mux_2_select_a() {
        let (_, tr) = before_mux_2();

        clear!(tr[SEL]);
        assert!(
            high!(tr[Y2]),
            "Y2 should be high when A2 is low and SEL is low"
        );

        set!(tr[A2]);
        assert!(
            low!(tr[Y2]),
            "Y2 should be low when A2 is high and SEL is low"
        )
    }

    #[test]
    fn mux_2_select_b() {
        let (_, tr) = before_mux_2();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y2]),
            "Y2 should be low when B2 is high and SEL is high"
        );

        clear!(tr[B2]);
        assert!(
            high!(tr[Y2]),
            "Y2 should be high when B2 is low and SEL is high"
        );
    }

    #[test]
    fn mux_2_oe_high() {
        let (_, tr) = before_mux_2();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y2]),
            "Y2 should be low when B2 is high and SEL is high"
        );

        set!(tr[OE]);
        assert!(floating!(tr[Y2]), "Y2 should float when OE is high");

        clear!(tr[SEL]);
        assert!(floating!(tr[Y2]), "Y2 should float when OE is high");
    }

    fn before_mux_3() -> (DeviceRef, RefVec<Trace>) {
        let (chip, tr) = before_each();
        clear!(tr[A3]);
        set!(tr[B3]);
        (chip, tr)
    }

    #[test]
    fn mux_3_select_a() {
        let (_, tr) = before_mux_3();

        clear!(tr[SEL]);
        assert!(
            high!(tr[Y3]),
            "Y3 should be high when A3 is low and SEL is low"
        );

        set!(tr[A3]);
        assert!(
            low!(tr[Y3]),
            "Y3 should be low when A3 is high and SEL is low"
        )
    }

    #[test]
    fn mux_3_select_b() {
        let (_, tr) = before_mux_3();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y3]),
            "Y3 should be low when B3 is high and SEL is high"
        );

        clear!(tr[B3]);
        assert!(
            high!(tr[Y3]),
            "Y3 should be high when B3 is low and SEL is high"
        );
    }

    #[test]
    fn mux_3_oe_high() {
        let (_, tr) = before_mux_3();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y3]),
            "Y3 should be low when B3 is high and SEL is high"
        );

        set!(tr[OE]);
        assert!(floating!(tr[Y3]), "Y3 should float when OE is high");

        clear!(tr[SEL]);
        assert!(floating!(tr[Y3]), "Y3 should float when OE is high");
    }

    fn before_mux_4() -> (DeviceRef, RefVec<Trace>) {
        let (chip, tr) = before_each();
        clear!(tr[A4]);
        set!(tr[B4]);
        (chip, tr)
    }

    #[test]
    fn mux_4_select_a() {
        let (_, tr) = before_mux_4();

        clear!(tr[SEL]);
        assert!(
            high!(tr[Y4]),
            "Y4 should be high when A4 is low and SEL is low"
        );

        set!(tr[A4]);
        assert!(
            low!(tr[Y4]),
            "Y4 should be low when A4 is high and SEL is low"
        )
    }

    #[test]
    fn mux_4_select_b() {
        let (_, tr) = before_mux_4();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y4]),
            "Y4 should be low when B4 is high and SEL is high"
        );

        clear!(tr[B4]);
        assert!(
            high!(tr[Y4]),
            "Y4 should be high when B4 is low and SEL is high"
        );
    }

    #[test]
    fn mux_4_oe_high() {
        let (_, tr) = before_mux_4();

        set!(tr[SEL]);
        assert!(
            low!(tr[Y4]),
            "Y4 should be low when B4 is high and SEL is high"
        );

        set!(tr[OE]);
        assert!(floating!(tr[Y4]), "Y4 should float when OE is high");

        clear!(tr[SEL]);
        assert!(floating!(tr[Y4]), "Y4 should float when OE is high");
    }
}
