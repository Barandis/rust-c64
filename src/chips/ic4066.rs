// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// The pin assignment for the first I/O pin of switch 1.
    pub const A1: usize = 1;
    /// The pin assignment for the second I/O pin of switch 1.
    pub const B1: usize = 2;
    /// The pin assignment for the control pin of switch 1.
    pub const X1: usize = 13;

    /// The pin assignment for the first I/O pin of switch 2.
    pub const A2: usize = 3;
    /// The pin assignment for the second I/O pin of switch 2.
    pub const B2: usize = 4;
    /// The pin assignment for the control pin of switch 2.
    pub const X2: usize = 5;

    /// The pin assignment for the first I/O pin of switch 3.
    pub const A3: usize = 9;
    /// The pin assignment for the second I/O pin of switch 3.
    pub const B3: usize = 8;
    /// The pin assignment for the control pin of switch 3.
    pub const X3: usize = 6;

    /// The pin assignment for the first I/O pin of switch 4.
    pub const A4: usize = 11;
    /// The pin assignment for the second I/O pin of switch 4.
    pub const B4: usize = 10;
    /// The pin assignment for the control pin of switch 4.
    pub const X4: usize = 12;

    /// The pin assignment for the +5V power supply.
    pub const VDD: usize = 14;
    /// The pin assignment for the ground.
    pub const VSS: usize = 7;
}

use crate::{
    components::{
        device::{Device, DeviceRef, LevelChange},
        pin::{
            Mode::{Bidirectional, Input, Unconnected},
            Pin, PinRef,
        },
    },
    ref_vec::RefVec,
    utils::value_high,
};

use self::constants::*;

const IOS: [usize; 8] = [A1, A2, A3, A4, B1, B2, B3, B4];
const CONTROLS: [usize; 4] = [X1, X2, X3, X4];

/// An emulation of the 4066 quad bilateral switch.
///
/// The 4066 is one of the 4000-series CMOS logic chips, consisting of four symmetrical
/// analog switches. The data pins transfer data bidirectionally as long as their associated
/// control pin is low. When the control pin goes high, no data can be passed through the
/// switch.
///
/// When the control pin returns to low, both data pins return to the level of the *last of
/// them to be set*. This is a bit of a compromise necessitated by the fact that this is a
/// digital simulation of an analog circuit, but it should be the most natural. Most use
/// cases do not involve switching the direction that data flows through the switch
/// regularly.
///
/// There is no high-impedance state for the pins of this device. When the control pin his
/// high, the data pins simply take on the level of whatever circuits they're connected to.
/// This is emulated by changing their mode to `INPUT` so that they do not send signals but
/// can still track changes on their traces.
///
/// There is no consistency across datahsheets for naming the 4066's pins. Many sheets
/// simply have some data pins marked "IN/OUT" and others marked "OUT/IN", but those don't
/// work well as property names. For consistency with the rest of the logic chips in this
/// module, the data pins have been named A and B, while thie control pin is named X. The A
/// and B pins are completely interchangeable and do appear in different orders oon many
/// datasheets; this particular arrangement (if not the pin names) is taken from the
/// datasheet for the Texas Instruments CD4066B.
///
/// The chip comes in a 14-pin dual in-line package with the following pin assignments.
/// ```text
///         +---+--+---+
///      A1 |1  +--+ 14| VDD
///      B1 |2       13| X1
///      B2 |3       12| X4
///      A2 |4  4066 11| A4
///      X2 |5       10| B4
///      X3 |6        9| B3
///     VSS |7        8| A3
///         +----------+
/// ```
/// VDD and VSS are power supply pins and are not emulated.
///
/// This chip is unusual in that it's the only analog chip in the system as emulated (with
/// the exception of the filter portion of the 6581). Even so, it works fine for switching
/// digital signals as well, and one of the Commodore 64's two 4066's is in fact used as a
/// digital switch.
///
/// In the Commodore 64, U16 and U28 are 4066's. The former is used as a digital switch to
/// control which processor has access to the color RAM's data pins, while the other is used
/// as an analog switch to control which game port is providing paddle data to the 6581 SID.
pub struct Ic4066 {
    /// The pins of the 4066, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,

    /// The index of the last I/O pin whose value has changed. There are four of these in
    /// this vector, one for each switch. These values are used to know what value to set
    /// the I/O pins to when the control pin transitions low.
    last: Vec<Option<usize>>,
}

impl Ic4066 {
    /// Creates a new 4066 quad bilateral switch emulation and returns a shared, internally
    /// mutable reference to it.
    pub fn new() -> DeviceRef {
        // I/O and control pins for switch 1
        let a1 = pin!(A1, "A1", Bidirectional);
        let b1 = pin!(B1, "B1", Bidirectional);
        let x1 = pin!(X1, "X1", Input);

        // I/O and control pins for switch 2
        let a2 = pin!(A2, "A2", Bidirectional);
        let b2 = pin!(B2, "B2", Bidirectional);
        let x2 = pin!(X2, "X2", Input);

        // I/O and control pins for switch 3
        let a3 = pin!(A3, "A3", Bidirectional);
        let b3 = pin!(B3, "B3", Bidirectional);
        let x3 = pin!(X3, "X3", Input);

        // I/O and control pins for switch 4
        let a4 = pin!(A4, "A4", Bidirectional);
        let b4 = pin!(B4, "B4", Bidirectional);
        let x4 = pin!(X4, "X4", Input);

        // Power supply and ground pins, not emulated
        let vss = pin!(VSS, "VSS", Unconnected);
        let vdd = pin!(VDD, "VDD", Unconnected);

        let last = vec![None, None, None, None];

        let chip: DeviceRef = new_ref!(Ic4066 {
            pins: pins![a1, a2, a3, a4, b1, b2, b3, b4, x1, x2, x3, x4, vdd, vss],
            last,
        });

        attach!(a1, clone_ref!(chip));
        attach!(b1, clone_ref!(chip));
        attach!(x1, clone_ref!(chip));
        attach!(a2, clone_ref!(chip));
        attach!(b2, clone_ref!(chip));
        attach!(x2, clone_ref!(chip));
        attach!(a3, clone_ref!(chip));
        attach!(b3, clone_ref!(chip));
        attach!(x3, clone_ref!(chip));
        attach!(a4, clone_ref!(chip));
        attach!(b4, clone_ref!(chip));
        attach!(x4, clone_ref!(chip));

        chip
    }
}

/// Maps each control pin assignment to a tuple of its switch's two I/O pin assignments.
fn ios_for(control: usize) -> (usize, usize) {
    match control {
        X1 => (A1, B1),
        X2 => (A2, B2),
        X3 => (A3, B3),
        X4 => (A4, B4),
        _ => (0, 0),
    }
}

/// Maps each I/O pin assignment to a tuple of its switch's other I/O pin assignment and
/// its switch's control pin assignment.
fn io_control_for(io: usize) -> (usize, usize) {
    match io {
        A1 => (B1, X1),
        B1 => (A1, X1),
        A2 => (B2, X2),
        B2 => (A2, X2),
        A3 => (B3, X3),
        B3 => (A3, X3),
        A4 => (B4, X4),
        B4 => (A4, X4),
        _ => (0, 0),
    }
}

/// Returns the index into the self.last vector for a control pin assignment.
fn switch(control: usize) -> usize {
    match control {
        X1 => 0,
        X2 => 1,
        X3 => 2,
        X4 => 3,
        _ => 4,
    }
}

impl Device for Ic4066 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        match event {
            // Control pin change
            LevelChange(pin, _, level) if CONTROLS.contains(&number!(pin)) => {
                let (a, b) = ios_for(number!(pin));
                let apin = clone_ref!(self.pins[a]);
                let bpin = clone_ref!(self.pins[b]);

                if value_high(*level) {
                    // Control pin high: change I/O pins to Input mode so that they don't
                    // broadcast data but can receive it to be stored
                    set_mode!(apin, Input);
                    set_mode!(bpin, Input);
                } else {
                    // Control pin low: change I/O pins to Bidirectional mode and set the
                    // level of the least-recently changed pin to that of the most-recently
                    // changed
                    set_mode!(apin, Bidirectional);
                    set_mode!(bpin, Bidirectional);

                    let index = switch(number!(pin));
                    match self.last[index] {
                        Some(num) if num == a => set_level!(bpin, level!(apin)),
                        Some(num) if num == b => set_level!(apin, level!(bpin)),
                        _ => {
                            clear!(apin);
                            clear!(bpin);
                        }
                    }
                }
            }
            // I/O pin change: remember the index of the pin being changed, and if the
            // control pin is low, set the level of the associated I/O pin to the new level
            LevelChange(pin, _, level) if IOS.contains(&number!(pin)) => {
                let (out, x) = io_control_for(number!(pin));
                let index = switch(x);

                self.last[index] = Some(number!(pin));
                if low!(self.pins[x]) {
                    set_level!(self.pins[out], *level);
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
        let chip = Ic4066::new();
        let tr = make_traces(clone_ref!(chip));
        (chip, tr)
    }

    #[test]
    fn pass_a_to_b() {
        let (_, tr) = before_each();

        clear!(tr[X1]);
        set_level!(tr[A1], Some(0.5));
        assert_eq!(level!(tr[B1]).unwrap(), 0.5, "B1's level should match A1's");

        clear!(tr[X2]);
        set_level!(tr[A2], Some(0.75));
        assert_eq!(
            level!(tr[B2]).unwrap(),
            0.75,
            "B2's level should match A2's"
        );

        clear!(tr[X3]);
        set_level!(tr[A3], Some(0.25));
        assert_eq!(
            level!(tr[B3]).unwrap(),
            0.25,
            "B3's level should match A3's"
        );

        clear!(tr[X4]);
        set_level!(tr[A4], Some(1.0));
        assert_eq!(level!(tr[B4]).unwrap(), 1.0, "B4's level should match A4's");
    }

    #[test]
    fn pass_b_to_a() {
        let (_, tr) = before_each();

        clear!(tr[X1]);
        set_level!(tr[B1], Some(0.5));
        assert_eq!(level!(tr[A1]).unwrap(), 0.5, "A1's level should match B1's");

        clear!(tr[X2]);
        set_level!(tr[B2], Some(0.75));
        assert_eq!(
            level!(tr[A2]).unwrap(),
            0.75,
            "A2's level should match B2's"
        );

        clear!(tr[X3]);
        set_level!(tr[B3], Some(0.25));
        assert_eq!(
            level!(tr[A3]).unwrap(),
            0.25,
            "A3's level should match B3's"
        );

        clear!(tr[X4]);
        set_level!(tr[B4], Some(1.0));
        assert_eq!(level!(tr[A4]).unwrap(), 1.0, "A4's level should match B4's");
    }

    #[test]
    fn disconnect_on_high_x() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        assert!(floating!(tr[A1]), "A1 should be disconnected");
        assert!(floating!(tr[B1]), "B1 should be disconnected");

        set!(tr[X2]);
        assert!(floating!(tr[A2]), "A2 should be disconnected");
        assert!(floating!(tr[B2]), "B2 should be disconnected");

        set!(tr[X3]);
        assert!(floating!(tr[A3]), "A3 should be disconnected");
        assert!(floating!(tr[B3]), "B3 should be disconnected");

        set!(tr[X4]);
        assert!(floating!(tr[A4]), "A4 should be disconnected");
        assert!(floating!(tr[B4]), "B4 should be disconnected");
    }

    #[test]
    fn no_pass_a_to_b_on_high_x() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        set_level!(tr[A1], Some(0.5));
        assert!(floating!(tr[B1]), "B1's level should not be affected by A1");

        set!(tr[X2]);
        set_level!(tr[A2], Some(0.75));
        assert!(floating!(tr[B2]), "B2's level should not be affected by A2");

        set!(tr[X3]);
        set_level!(tr[A3], Some(0.25));
        assert!(floating!(tr[B3]), "B3's level should not be affected by A3");

        set!(tr[X4]);
        set_level!(tr[A4], Some(1.0));
        assert!(floating!(tr[B4]), "B4's level should not be affected by A4");
    }

    #[test]
    fn no_pass_b_to_a_on_high_x() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        set_level!(tr[B1], Some(0.5));
        assert!(floating!(tr[A1]), "A1's level should not be affected by B1");

        set!(tr[X2]);
        set_level!(tr[B2], Some(0.75));
        assert!(floating!(tr[A2]), "A2's level should not be affected by B2");

        set!(tr[X3]);
        set_level!(tr[B3], Some(0.25));
        assert!(floating!(tr[A3]), "A3's level should not be affected by B3");

        set!(tr[X4]);
        set_level!(tr[B4], Some(1.0));
        assert!(floating!(tr[A4]), "A4's level should not be affected by B4");
    }

    #[test]
    fn last_set_a() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        set_level!(tr[B1], Some(1.5));
        set_level!(tr[A1], Some(0.5));
        clear!(tr[X1]);
        assert_eq!(
            level!(tr[B1]).unwrap(),
            0.5,
            "B1's level should match A1's since it was last set"
        );

        set!(tr[X2]);
        set_level!(tr[B2], Some(1.5));
        set_level!(tr[A2], Some(0.75));
        clear!(tr[X2]);
        assert_eq!(
            level!(tr[B2]).unwrap(),
            0.75,
            "B2's level should match A2's since it was last set"
        );

        set!(tr[X3]);
        set_level!(tr[B3], Some(1.5));
        set_level!(tr[A3], Some(0.25));
        clear!(tr[X3]);
        assert_eq!(
            level!(tr[B3]).unwrap(),
            0.25,
            "B3's level should match A3's since it was last set"
        );

        set!(tr[X4]);
        set_level!(tr[B4], Some(1.5));
        set_level!(tr[A4], Some(1.0));
        clear!(tr[X4]);
        assert_eq!(
            level!(tr[B4]).unwrap(),
            1.0,
            "B4's level should match A4's since it was last set"
        );
    }

    #[test]
    fn last_set_b() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        set_level!(tr[A1], Some(1.5));
        set_level!(tr[B1], Some(0.5));
        clear!(tr[X1]);
        assert_eq!(
            level!(tr[A1]).unwrap(),
            0.5,
            "A1's level should match B1's since it was last set"
        );

        set!(tr[X2]);
        set_level!(tr[A2], Some(1.5));
        set_level!(tr[B2], Some(0.75));
        clear!(tr[X2]);
        assert_eq!(
            level!(tr[A2]).unwrap(),
            0.75,
            "A2's level should match B2's since it was last set"
        );

        set!(tr[X3]);
        set_level!(tr[A3], Some(1.5));
        set_level!(tr[B3], Some(0.25));
        clear!(tr[X3]);
        assert_eq!(
            level!(tr[A3]).unwrap(),
            0.25,
            "A3's level should match B3's since it was last set"
        );

        set!(tr[X4]);
        set_level!(tr[A4], Some(1.5));
        set_level!(tr[B4], Some(1.0));
        clear!(tr[X4]);
        assert_eq!(
            level!(tr[A4]).unwrap(),
            1.0,
            "A4's level should match B4's since it was last set"
        );
    }

    #[test]
    fn unset_before_high_x() {
        let (_, tr) = before_each();

        set!(tr[X1]);
        clear!(tr[X1]);
        assert_eq!(
            level!(tr[A1]).unwrap(),
            0.0,
            "A1 should be low since nothing was last set"
        );
        assert_eq!(
            level!(tr[B1]).unwrap(),
            0.0,
            "B1 should be low since nothing was last set"
        );

        set!(tr[X2]);
        clear!(tr[X2]);
        assert_eq!(
            level!(tr[A2]).unwrap(),
            0.0,
            "A2 should be low since nothing was last set"
        );
        assert_eq!(
            level!(tr[B2]).unwrap(),
            0.0,
            "B2 should be low since nothing was last set"
        );

        set!(tr[X3]);
        clear!(tr[X3]);
        assert_eq!(
            level!(tr[A3]).unwrap(),
            0.0,
            "A3 should be low since nothing was last set"
        );
        assert_eq!(
            level!(tr[B3]).unwrap(),
            0.0,
            "B3 should be low since nothing was last set"
        );

        set!(tr[X4]);
        clear!(tr[X4]);
        assert_eq!(
            level!(tr[A4]).unwrap(),
            0.0,
            "A4 should be low since nothing was last set"
        );
        assert_eq!(
            level!(tr[B4]).unwrap(),
            0.0,
            "B4 should be low since nothing was last set"
        );
    }
}
