// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// Pin assignment for input pin 0.
    pub const D0: usize = 3;
    /// Pin assignment for input pin 1.
    pub const D1: usize = 4;
    /// Pin assignment for input pin 2.
    pub const D2: usize = 7;
    /// Pin assignment for input pin 3.
    pub const D3: usize = 8;
    /// Pin assignment for input pin 4.
    pub const D4: usize = 13;
    /// Pin assignment for input pin 5.
    pub const D5: usize = 14;
    /// Pin assignment for input pin 6.
    pub const D6: usize = 17;
    /// Pin assignment for input pin 7.
    pub const D7: usize = 18;

    /// Pin assignment for output pin 0.
    pub const Q0: usize = 2;
    /// Pin assignment for output pin 1.
    pub const Q1: usize = 5;
    /// Pin assignment for output pin 2.
    pub const Q2: usize = 6;
    /// Pin assignment for output pin 3.
    pub const Q3: usize = 9;
    /// Pin assignment for output pin 4.
    pub const Q4: usize = 12;
    /// Pin assignment for output pin 5.
    pub const Q5: usize = 15;
    /// Pin assignment for output pin 6.
    pub const Q6: usize = 16;
    /// Pin assignment for output pin 7.
    pub const Q7: usize = 19;

    /// Pin assignment for the output enable pin.
    pub const OE: usize = 1;
    /// Pin assignment for the latch enable pin.
    pub const LE: usize = 11;

    /// Pin assignment for the +5V power supply.
    pub const VCC: usize = 20;
    /// Pin assignment for the ground.
    pub const GND: usize = 10;
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

const INPUTS: [usize; 8] = [D0, D1, D2, D3, D4, D5, D6, D7];
const OUTPUTS: [usize; 8] = [Q0, Q1, Q2, Q3, Q4, Q5, Q6, Q7];

/// An emulation of the 74373 octal D-type transparent latch.
///
/// The 74373 is one of the 7400-series TTL logic chips, consisting of eight transparent
/// latches. These latches normally allow data to flow freely from input to output, but when
/// the latch enable pin `LE` is set to low, the output is latched. That means it retains
/// its current state, no matter what the input pins do in the meantime. Once `LE` goes high
/// again, the outputs once more reflect their inputs.
///
/// Since this chip is most often used in bus-type applications, the pins are named using
/// more of a bus-type convention. The inputs are D and the outputs are Q, and the latches
/// are numbered from 0 rather than from 1.
///
/// The chip has an active-low output enable pin, OE. When this is high, all outputs are set
/// to a high impedance state.
///
/// | OE    | LE    | Dn    | Qn    |
/// | :---: | :---: | :---: | :---: |
/// | H     | X     | X     | **Z** |
/// | L     | H     | L     | **L** |
/// | L     | H     | H     | **H** |
/// | L     | L     | X     | **Q₀**|
///
/// Q₀ means whatever level the pin was in the previous state. If the pin was high, then it
/// remains high. If it was low, it remains low.
///
/// The chip comes in a 20-pin dual in-line package with the following pin assignments.
/// ```text
///         +---+--+---+
///      OE |1  +--+ 20| VCC
///      Q0 |2       19| Q7
///      D0 |3       18| D7
///      D1 |4       17| D6
///      Q1 |5       16| Q6
///      Q2 |6 74373 15| Q5
///      D2 |7       14| D5
///      D3 |8       13| D4
///      Q3 |9       12| Q4
///     GND |10      11| LE
///         +----------+
/// ```
/// GND and VCC are ground and power supply pins respectively, and they are not emulated.
///
/// In the Commodore 64, U26 is a 74LS373 (a lower-power, faster variant whose emulation is
/// the same). It's used to connect the multiplexed address bus to the lower 8 bits of the
/// main address bus. It latches the low 8 bits of the multiplexed bus so that, when the
/// lines are switched to the high 8 bits, those bits do not leak onto the low 8 bits of the
/// main bus.
pub struct Ic74373 {
    /// The pins of the 74373, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,

    /// The latched output values for each output pin. When the outputs are not being
    /// latched, all of the values here will be `None`.
    latches: Vec<Option<f64>>,
}

impl Ic74373 {
    pub fn new() -> DeviceRef {
        // Input pins
        let d0 = pin!(D0, "D0", Input);
        let d1 = pin!(D1, "D1", Input);
        let d2 = pin!(D2, "D2", Input);
        let d3 = pin!(D3, "D3", Input);
        let d4 = pin!(D4, "D4", Input);
        let d5 = pin!(D5, "D5", Input);
        let d6 = pin!(D6, "D6", Input);
        let d7 = pin!(D7, "D7", Input);

        // Output pins
        let q0 = pin!(Q0, "Q0", Output);
        let q1 = pin!(Q1, "Q1", Output);
        let q2 = pin!(Q2, "Q2", Output);
        let q3 = pin!(Q3, "Q3", Output);
        let q4 = pin!(Q4, "Q4", Output);
        let q5 = pin!(Q5, "Q5", Output);
        let q6 = pin!(Q6, "Q6", Output);
        let q7 = pin!(Q7, "Q7", Output);

        // Output enable. When this is high, the outputs function normally according to
        // their inputs and LE. When this is low, the outputs are all hi-Z.
        let oe = pin!(OE, "OE", Input);

        // Latch enable. When set high, data flows transparently through the device, with
        // output pins matching their input pins. When it goes low, the output pins remain
        // in their current state for as long as LE is low, no matter what the inputs do.
        let le = pin!(LE, "LE", Input);

        // Power supply and ground pins, not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let chip: DeviceRef = new_ref!(Ic74373 {
            pins: pins![
                d0, d1, d2, d3, d4, d5, d6, d7, q0, q1, q2, q3, q4, q5, q6, q7, oe, le, vcc, gnd
            ],
            latches: vec![None; 8],
        });

        clear!(q0, q1, q2, q3, q4, q5, q6, q7);

        attach!(d0, clone_ref!(chip));
        attach!(d1, clone_ref!(chip));
        attach!(d2, clone_ref!(chip));
        attach!(d3, clone_ref!(chip));
        attach!(d4, clone_ref!(chip));
        attach!(d5, clone_ref!(chip));
        attach!(d6, clone_ref!(chip));
        attach!(d7, clone_ref!(chip));
        attach!(oe, clone_ref!(chip));
        attach!(le, clone_ref!(chip));

        chip
    }
}

/// Maps each input pin assignment to its corresponding output pin assignm,ent.
fn output_for(input: usize) -> usize {
    match input {
        D0 => Q0,
        D1 => Q1,
        D2 => Q2,
        D3 => Q3,
        D4 => Q4,
        D5 => Q5,
        D6 => Q6,
        D7 => Q7,
        _ => 0,
    }
}

impl Device for Ic74373 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        match event {
            LevelChange(pin, _, level) if INPUTS.contains(&number!(pin)) => {
                if high!(self.pins[LE]) && !high!(self.pins[OE]) {
                    let q = output_for(number!(pin));
                    if value_high(*level) {
                        set!(self.pins[q]);
                    } else {
                        clear!(self.pins[q]);
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == LE => {
                if value_high(*level) {
                    for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
                        let q = output_for(d);
                        if value_high(level!(self.pins[d])) {
                            set!(self.pins[q]);
                        } else {
                            clear!(self.pins[q]);
                        }
                        self.latches[i] = None;
                    }
                } else {
                    for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
                        self.latches[i] = if value_high(level!(self.pins[d])) {
                            Some(1.0)
                        } else {
                            Some(0.0)
                        };
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == OE => {
                if value_high(*level) {
                    for q in OUTPUTS {
                        float!(self.pins[q]);
                    }
                } else {
                    let latched = !high!(self.pins[LE]);
                    for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
                        let q = output_for(d);
                        if latched {
                            set_level!(self.pins[q], self.latches[i]);
                        } else if value_high(level!(self.pins[d])) {
                            set!(self.pins[q]);
                        } else {
                            clear!(self.pins[q]);
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{components::trace::TraceRef, test_utils::make_traces};

    use super::*;

    fn before_each() -> (DeviceRef, Vec<TraceRef>) {
        let chip = Ic74373::new();
        let tr = make_traces(clone_ref!(chip));
        set!(tr[LE]);
        clear!(tr[OE]);
        (chip, tr)
    }

    #[test]
    fn pass_high_le() {
        let (_, tr) = before_each();

        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            let q = output_for(d);
            set!(tr[d]);
            assert!(
                high!(tr[q]),
                "Q{0} should be high when LE is high and D{0} is high",
                i
            );
        }

        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            let q = output_for(d);
            clear!(tr[d]);
            assert!(
                low!(tr[q]),
                "Q{0} should be low when LE is high and D{0} is low",
                i
            );
        }
    }

    #[test]
    fn latch_low_le() {
        let (_, tr) = before_each();

        // Sets outputs to 01010101 (Q7-Q0)
        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            set_level!(tr[d], Some(((i + 1) % 2) as f64));
        }

        clear!(tr[LE]);

        // Odd outputs remain low even when inputs are all set high
        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            let q = output_for(d);
            set!(tr[d]);
            assert_eq!(
                level!(tr[q]).unwrap(),
                ((i + 1) % 2) as f64,
                "Q{} should remain {} when LE is low",
                i,
                if i % 2 == 0 { "high" } else { "low" }
            );
        }
        // Even outputs remain high even when inputs are set low
        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            let q = output_for(d);
            clear!(tr[d]);
            assert_eq!(
                level!(tr[q]).unwrap(),
                ((i + 1) % 2) as f64,
                "Q{} should remain {} when LE is low",
                i,
                if i % 2 == 0 { "high" } else { "low" }
            );
        }
    }

    #[test]
    fn pass_return_to_high_le() {
        let (_, tr) = before_each();

        // Sets outputs to 01010101 (Q7-Q0)
        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            set_level!(tr[d], Some(((i + 1) % 2) as f64));
        }

        clear!(tr[LE]);

        for (i, d) in IntoIterator::into_iter(INPUTS).enumerate() {
            let q = output_for(d);
            // All inputs are set high here
            set!(tr[d]);
            assert_eq!(
                level!(tr[q]).unwrap(),
                ((i + 1) % 2) as f64,
                "Q{} should remain {} when LE is low",
                i,
                if i % 2 == 0 { "high" } else { "low" }
            );
        }

        set!(tr[LE]);

        // Outputs immediately switch to input value, which is still all high
        for (i, q) in IntoIterator::into_iter(OUTPUTS).enumerate() {
            assert!(
                high!(tr[q]),
                "Q{0} should be high when LE is high and D{0} is high",
                i
            )
        }
    }

    #[test]
    fn float_high_oe() {
        let (_, tr) = before_each();

        for d in INPUTS {
            set!(tr[d]);
        }

        set!(tr[OE]);

        for (i, q) in IntoIterator::into_iter(OUTPUTS).enumerate() {
            assert!(floating!(tr[q]), "Q{} should float when OE is high", i);
        }

        clear!(tr[OE]);

        for (i, q) in IntoIterator::into_iter(OUTPUTS).enumerate() {
            assert!(
                high!(tr[q]),
                "Q{0} should be high when LE is high and D{0} is high",
                i
            );
        }
    }

    #[test]
    fn recall_latch_high_oe() {
        let (_, tr) = before_each();

        for d in INPUTS {
            set!(tr[d]);
        }

        set!(tr[OE]);

        for d in [D0, D2, D4, D6] {
            clear!(tr[d]);
        }
        clear!(tr[LE]);

        clear!(tr[OE]);

        for (i, q) in IntoIterator::into_iter(OUTPUTS).enumerate() {
            assert_eq!(
                level!(tr[q]).unwrap(),
                (i % 2) as f64,
                "Q{} should remain {} when LE is low",
                i,
                if i % 2 == 0 { "low" } else { "high" }
            )
        }
    }
}
