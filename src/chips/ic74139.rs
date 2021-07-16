// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// The pin assignment for the low input pin on demultiplexer 1.
    pub const A1: usize = 2;
    /// The pin assignment for the high input pin on demultiplexer 1.
    pub const B1: usize = 3;
    /// The pin assignment for the first output pin on demultiplexer 1.
    pub const Y10: usize = 4;
    /// The pin assignment for the second output pin on demultiplexer 1.
    pub const Y11: usize = 5;
    /// The pin assignment for the third output pin on demultiplexer 1.
    pub const Y12: usize = 6;
    /// The pin assignment for the fourth output pin on demultiplexer 1.
    pub const Y13: usize = 7;
    /// The pin assignment for the enable pin on demultiplexer 1.
    pub const G1: usize = 1;

    /// The pin assignment for the low input pin on demultiplexer 2.
    pub const A2: usize = 14;
    /// The pin assignment for the high input pin on demultiplexer 2.
    pub const B2: usize = 13;
    /// The pin assignment for the first output pin on demultiplexer 2.
    pub const Y20: usize = 12;
    /// The pin assignment for the second output pin on demultiplexer 2.
    pub const Y21: usize = 11;
    /// The pin assignment for the third output pin on demultiplexer 2.
    pub const Y22: usize = 10;
    /// The pin assignment for the fourth output pin on demultiplexer 2.
    pub const Y23: usize = 9;
    /// The pin assignment for the enable pin on demultiplexer 2.
    pub const G2: usize = 15;

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

// /An emulation of the 74139 dual 2-to-4 demultiplexer.
///
/// The 74139 is one of the 7400-series TTL logic chips, consisting of a pair of 2-input,
/// 4-output demultiplexers. There are four possible binary combinations on two pins (LL,
/// HL, LH, and HH), and each of these combinations selects a different one of the output
/// pins to activate. Each demultiplexer also has an enable pin.
///
/// Most literature names the pins with numbers first. This makes sense since there are
/// really two numbers that go into the output's name (the demultiplexer number and the
/// output number) and having a letter separate them is quite readable. But since each of
/// these pin names becomes the name of a constant, that scheme cannot be used here.
/// Therefore each demultiplexer has two inputs starting with A and B, an active-low enable
/// pin starting with G, and four inverted outputs whose names start with Y.
///
/// | Gn    | An    | Bn    | Yn0   | Yn1   | Yn2   | Yn3   |
/// | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
/// | H     | X     | X     | **H** | **H** | **H** | **H** |
/// | L     | L     | L     | **L** | **H** | **H** | **H** |
/// | L     | H     | L     | **H** | **L** | **H** | **H** |
/// | L     | L     | H     | **H** | **H** | **L** | **H** |
/// | L     | H     | H     | **H** | **H** | **H** | **L** |
///
/// In the Commodore 64, the two demultiplexers are chained together by connecting one of
/// the outputs from demux 1 to the enable pin of demux 2. The inputs are the address lines
/// A8-A11, and the enable pin of demux 1 comes directly from the PLA's IO output. Thus the
/// demultiplexers only do work when IO is selected, which requires that the address be from
/// $D000 - $DFFF, among other things. A more specific table for this setup can thus be
/// created.
///
/// | IO    | A8    | A9    | A10   | A11   | Address       | Active Output |
/// | :---: | :---: | :---: | :---: | :---: | ------------- | ------------- |
/// | H     | X     | X     | X     | X     | N/A           | **None**      |
/// | L     | X     | X     | L     | L     | $D000 - $D3FF | **VIC**       |
/// | L     | X     | X     | H     | L     | $D400 - $D7FF | **SID**       |
/// | L     | X     | X     | L     | H     | $D800 - $DBFF | **Color RAM** |
/// | L     | L     | L     | H     | H     | $DC00 - $DCFF | **CIA 1**     |
/// | L     | H     | L     | H     | H     | $DD00 - $DDFF | **CIA 2**     |
/// | L     | L     | H     | H     | H     | $DE00 - $DEFF | **I/O 1**     |
/// | L     | H     | H     | H     | H     | $DF00 - $DFFF | **I/O 2**     |
///
/// The decoding resolution is only 2 hexadecimal digits for the VIC, SID, and color RAM and
/// 3 hexadecimal digits for the CIAs and I/Os. This means that there will be memory
/// locations that repeat. For example, the VIC only uses 64 addressable locations for its
/// registers (47 registers and 17 more unused addresses) but gets a 1024-address block. The
/// decoding can't tell the difference between $D000, $D040, $D080, and so on because it can
/// only resolve the first two digits, so using any of those addresses will access the VIC's
/// first register, meaning that it's mirrored 16 times. The same goes for the SID (29
/// registers and 3 usused addresses, mirrored in 1024 addresses 32 times) and the CIAs (16
/// registers mirrored in 256 addresses 16 times). The color RAM is not mirrored at all
/// (though it does use only 1000 of its 1024 addresses) and the I/O blocks are free to be
/// managed by cartridges as they like.
///
/// The chip comes in a 16-pin dual in-line package with the following pin assignments.
/// ```text
///          +---+--+---+
///       G1 |1  +--+ 16| VCC
///       A1 |2       15| G2
///       B1 |3       14| A2
///      Y10 |4       13| B2
///      Y11 |5 74139 12| Y20
///      Y12 |6       11| Y21
///      Y13 |7       10| Y22
///      GND |8        9| Y23
///          +----------+
/// ```
/// GND and VCC are ground and power supply pins respectively, and they are not emulated.
///
/// In the Commodore 64, U15 is a 74LS139 (a lower-power, faster variant whose emulation is
/// the same). Its two demultiplexers are chained together to provide additional address
/// decoding when the PLA's IO output is selected.
pub struct Ic74139 {
    /// The pins of the 74139, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,
}

impl Ic74139 {
    /// Creates a new 74139 dual 2-to-4 demultiplexer emulation and returns a shared,
    /// internally mutable reference to it.
    pub fn new() -> DeviceRef {
        // Demultiplexer 1
        let a1 = pin!(A1, "A1", Input);
        let b1 = pin!(B1, "B1", Input);
        let y10 = pin!(Y10, "Y10", Output);
        let y11 = pin!(Y11, "Y11", Output);
        let y12 = pin!(Y12, "Y12", Output);
        let y13 = pin!(Y13, "Y13", Output);
        let g1 = pin!(G1, "G1", Input);

        // Demultiplexer 2
        let a2 = pin!(A2, "A2", Input);
        let b2 = pin!(B2, "B2", Input);
        let y20 = pin!(Y20, "Y20", Output);
        let y21 = pin!(Y21, "Y21", Output);
        let y22 = pin!(Y22, "Y22", Output);
        let y23 = pin!(Y23, "Y23", Output);
        let g2 = pin!(G2, "G2", Input);

        // Power supply and ground pins, not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let device: DeviceRef = new_ref!(Ic74139 {
            pins: pins![a1, a2, b1, b2, g1, g2, y10, y11, y12, y13, y20, y21, y22, y23, vcc, gnd]
        });

        set!(y11, y12, y13, y21, y22, y23);
        clear!(y10, y20);
        attach_to!(device, a1, a2, b1, b2, g1, g2);

        device
    }
}

/// Maps a control pin assignment to its associated two input pin assignemnts.
fn inputs(index: usize) -> (usize, usize) {
    match index {
        G1 => (A1, B1),
        G2 => (A2, B2),
        _ => (0, 0),
    }
}

/// Maps a control or input pin assignment to its associated four output pin assignments.
fn outputs(index: usize) -> (usize, usize, usize, usize) {
    match index {
        A1 | B1 | G1 => (Y10, Y11, Y12, Y13),
        A2 | B2 | G2 => (Y20, Y21, Y22, Y23),
        _ => (0, 0, 0, 0),
    }
}

/// Maps an input pin assignment to its associated other input pin assignment and control
/// pin assignment.
fn input_control_for(index: usize) -> (usize, usize) {
    match index {
        A1 => (B1, G1),
        B1 => (A1, G1),
        A2 => (B2, G2),
        B2 => (A2, G2),
        _ => (0, 0),
    }
}

impl Device for Ic74139 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        // Some macros to ease repitition (each of these is invoked three times in the
        // code below) and to provide some better clarity.
        //
        // They have to be defined inside the function as they use `self`, and macros can
        // only reference values explicitly passed in or which are defined in the place
        // where the macro is defined.
        macro_rules! ll {
            ($y0:expr, $y1:expr, $y2:expr, $y3:expr) => {
                clear!(self.pins[$y0]);
                set!(self.pins[$y1], self.pins[$y2], self.pins[$y3]);
            };
        }
        macro_rules! hl {
            ($y0:expr, $y1:expr, $y2:expr, $y3:expr) => {
                clear!(self.pins[$y1]);
                set!(self.pins[$y0], self.pins[$y2], self.pins[$y3]);
            };
        }
        macro_rules! lh {
            ($y0:expr, $y1:expr, $y2:expr, $y3:expr) => {
                clear!(self.pins[$y2]);
                set!(self.pins[$y0], self.pins[$y1], self.pins[$y3]);
            };
        }
        macro_rules! hh {
            ($y0:expr, $y1:expr, $y2:expr, $y3:expr) => {
                clear!(self.pins[$y3]);
                set!(self.pins[$y0], self.pins[$y1], self.pins[$y2]);
            };
        }

        match event {
            // We do split the arms for the A pin versus the B pin becuase we need to do
            // something different based on which one it is (HL for AB produces a different
            // output than LH, for example)
            LevelChange(pin, _, level) if number!(pin) == A1 || number!(pin) == A2 => {
                let (b, g) = input_control_for(number!(pin));
                let (y0, y1, y2, y3) = outputs(number!(pin));

                if high!(self.pins[g]) {
                    set!(self.pins[y0], self.pins[y1], self.pins[y2], self.pins[y3]);
                } else {
                    if value_high(*level) {
                        if high!(self.pins[b]) {
                            hh!(y0, y1, y2, y3);
                        } else {
                            hl!(y0, y1, y2, y3);
                        }
                    } else {
                        if high!(self.pins[b]) {
                            lh!(y0, y1, y2, y3);
                        } else {
                            ll!(y0, y1, y2, y3);
                        }
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == B1 || number!(pin) == B2 => {
                let (a, g) = input_control_for(number!(pin));
                let (y0, y1, y2, y3) = outputs(number!(pin));

                if high!(self.pins[g]) {
                    set!(self.pins[y0], self.pins[y1], self.pins[y2], self.pins[y3]);
                } else {
                    if value_high(*level) {
                        if high!(self.pins[a]) {
                            hh!(y0, y1, y2, y3);
                        } else {
                            lh!(y0, y1, y2, y3);
                        }
                    } else {
                        if high!(self.pins[a]) {
                            hl!(y0, y1, y2, y3);
                        } else {
                            ll!(y0, y1, y2, y3);
                        }
                    }
                }
            }
            LevelChange(pin, _, level) if number!(pin) == G1 || number!(pin) == G2 => {
                let (a, b) = inputs(number!(pin));
                let (y0, y1, y2, y3) = outputs(number!(pin));

                if value_high(*level) {
                    set!(self.pins[y0], self.pins[y1], self.pins[y2], self.pins[y3]);
                } else {
                    match (high!(self.pins[a]), high!(self.pins[b])) {
                        // These look like they can be expressions, but the macro expansions
                        // are not, hence the braces. We could put the macro expansion in
                        // braces, but that would mean an extra set of braces in the ef/else
                        // statements above.
                        (false, false) => {
                            ll!(y0, y1, y2, y3);
                        }
                        (true, false) => {
                            hl!(y0, y1, y2, y3);
                        }
                        (false, true) => {
                            lh!(y0, y1, y2, y3);
                        }
                        (true, true) => {
                            hh!(y0, y1, y2, y3);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{components::trace::Trace, test_utils::make_traces};

    use super::*;

    fn before_each() -> (DeviceRef, RefVec<Trace>) {
        let chip = Ic74139::new();
        let tr = make_traces(clone_ref!(chip));
        (chip, tr)
    }

    #[test]
    fn demux_1_g_high() {
        let (_, tr) = before_each();

        set!(tr[G1]);
        clear!(tr[A1]);
        clear!(tr[B1]);
        assert!(high!(tr[Y10]), "Y10 should be high when G1 is high");
        assert!(high!(tr[Y11]), "Y11 should be high when G1 is high");
        assert!(high!(tr[Y12]), "Y12 should be high when G1 is high");
        assert!(high!(tr[Y13]), "Y13 should be high when G1 is high");

        set!(tr[A1]);
        assert!(high!(tr[Y10]), "Y10 should be high when G1 is high");
        assert!(high!(tr[Y11]), "Y11 should be high when G1 is high");
        assert!(high!(tr[Y12]), "Y12 should be high when G1 is high");
        assert!(high!(tr[Y13]), "Y13 should be high when G1 is high");

        set!(tr[B1]);
        assert!(high!(tr[Y10]), "Y10 should be high when G1 is high");
        assert!(high!(tr[Y11]), "Y11 should be high when G1 is high");
        assert!(high!(tr[Y12]), "Y12 should be high when G1 is high");
        assert!(high!(tr[Y13]), "Y13 should be high when G1 is high");

        clear!(tr[A1]);
        assert!(high!(tr[Y10]), "Y10 should be high when G1 is high");
        assert!(high!(tr[Y11]), "Y11 should be high when G1 is high");
        assert!(high!(tr[Y12]), "Y12 should be high when G1 is high");
        assert!(high!(tr[Y13]), "Y13 should be high when G1 is high");
    }

    #[test]
    fn demux_1_low_low() {
        let (_, tr) = before_each();

        clear!(tr[G1]);
        clear!(tr[A1]);
        clear!(tr[B1]);
        assert!(
            low!(tr[Y10]),
            "Y10 should be low when A1 and B1 are both low"
        );
        assert!(
            high!(tr[Y11]),
            "Y11 should be high when A1 and B1 are both low"
        );
        assert!(
            high!(tr[Y12]),
            "Y12 should be high when A1 and B1 are both low"
        );
        assert!(
            high!(tr[Y13]),
            "Y13 should be high when A1 and B1 are both low"
        );
    }

    #[test]
    fn demux_1_high_low() {
        let (_, tr) = before_each();

        clear!(tr[G1]);
        set!(tr[A1]);
        clear!(tr[B1]);
        assert!(
            high!(tr[Y10]),
            "Y10 should be high when A1 is high and B1 is low"
        );
        assert!(
            low!(tr[Y11]),
            "Y11 should be low when A1 is high and B1 is low"
        );
        assert!(
            high!(tr[Y12]),
            "Y12 should be high when A1 is high and B1 is low"
        );
        assert!(
            high!(tr[Y13]),
            "Y13 should be high when A1 is high and B1 is low"
        );
    }

    #[test]
    fn demux_1_low_high() {
        let (_, tr) = before_each();

        clear!(tr[G1]);
        clear!(tr[A1]);
        set!(tr[B1]);
        assert!(
            high!(tr[Y10]),
            "Y10 should be high when A1 is low and B1 is high"
        );
        assert!(
            high!(tr[Y11]),
            "Y11 should be high when A1 is low and B1 is high"
        );
        assert!(
            low!(tr[Y12]),
            "Y12 should be low when A1 is low and B1 is high"
        );
        assert!(
            high!(tr[Y13]),
            "Y13 should be high when A1 is low and B1 is high"
        );
    }

    #[test]
    fn demux_1_high_high() {
        let (_, tr) = before_each();

        clear!(tr[G1]);
        set!(tr[A1]);
        set!(tr[B1]);
        assert!(
            high!(tr[Y10]),
            "Y10 should be high when A1 and B1 are both high"
        );
        assert!(
            high!(tr[Y11]),
            "Y11 should be high when A1 and B1 are both high"
        );
        assert!(
            high!(tr[Y12]),
            "Y12 should be high when A1 and B1 are both high"
        );
        assert!(
            low!(tr[Y13]),
            "Y13 should be low when A1 and B1 are both high"
        );
    }

    #[test]
    fn demux_2_g_high() {
        let (_, tr) = before_each();

        set!(tr[G2]);
        clear!(tr[A2]);
        clear!(tr[B2]);
        assert!(high!(tr[Y20]), "Y20 should be high when G2 is high");
        assert!(high!(tr[Y21]), "Y21 should be high when G2 is high");
        assert!(high!(tr[Y22]), "Y22 should be high when G2 is high");
        assert!(high!(tr[Y23]), "Y23 should be high when G2 is high");

        set!(tr[A2]);
        assert!(high!(tr[Y20]), "Y20 should be high when G2 is high");
        assert!(high!(tr[Y21]), "Y21 should be high when G2 is high");
        assert!(high!(tr[Y22]), "Y22 should be high when G2 is high");
        assert!(high!(tr[Y23]), "Y23 should be high when G2 is high");

        set!(tr[B2]);
        assert!(high!(tr[Y20]), "Y20 should be high when G2 is high");
        assert!(high!(tr[Y21]), "Y21 should be high when G2 is high");
        assert!(high!(tr[Y22]), "Y22 should be high when G2 is high");
        assert!(high!(tr[Y23]), "Y23 should be high when G2 is high");

        clear!(tr[A2]);
        assert!(high!(tr[Y20]), "Y20 should be high when G2 is high");
        assert!(high!(tr[Y21]), "Y21 should be high when G2 is high");
        assert!(high!(tr[Y22]), "Y22 should be high when G2 is high");
        assert!(high!(tr[Y23]), "Y23 should be high when G2 is high");
    }

    #[test]
    fn demux_2_low_low() {
        let (_, tr) = before_each();

        clear!(tr[G2]);
        clear!(tr[A2]);
        clear!(tr[B2]);
        assert!(
            low!(tr[Y20]),
            "Y20 should be low when A2 and B2 are both low"
        );
        assert!(
            high!(tr[Y21]),
            "Y21 should be high when A2 and B2 are both low"
        );
        assert!(
            high!(tr[Y22]),
            "Y22 should be high when A2 and B2 are both low"
        );
        assert!(
            high!(tr[Y23]),
            "Y23 should be high when A2 and B2 are both low"
        );
    }

    #[test]
    fn demux_2_high_low() {
        let (_, tr) = before_each();

        clear!(tr[G2]);
        set!(tr[A2]);
        clear!(tr[B2]);
        assert!(
            high!(tr[Y20]),
            "Y10 should be high when A2 is high and B2 is low"
        );
        assert!(
            low!(tr[Y21]),
            "Y11 should be low when A2 is high and B2 is low"
        );
        assert!(
            high!(tr[Y22]),
            "Y12 should be high when A2 is high and B2 is low"
        );
        assert!(
            high!(tr[Y23]),
            "Y13 should be high when A2 is high and B2 is low"
        );
    }

    #[test]
    fn demux_2_low_high() {
        let (_, tr) = before_each();

        clear!(tr[G2]);
        clear!(tr[A2]);
        set!(tr[B2]);
        assert!(
            high!(tr[Y20]),
            "Y20 should be high when A2 is low and B2 is high"
        );
        assert!(
            high!(tr[Y21]),
            "Y21 should be high when A2 is low and B2 is high"
        );
        assert!(
            low!(tr[Y22]),
            "Y22 should be low when A2 is low and B2 is high"
        );
        assert!(
            high!(tr[Y23]),
            "Y23 should be high when A2 is low and B2 is high"
        );
    }

    #[test]
    fn demux_2_high_high() {
        let (_, tr) = before_each();

        clear!(tr[G2]);
        set!(tr[A2]);
        set!(tr[B2]);
        assert!(
            high!(tr[Y20]),
            "Y20 should be high when A2 and B2 are both high"
        );
        assert!(
            high!(tr[Y21]),
            "Y21 should be high when A2 and B2 are both high"
        );
        assert!(
            high!(tr[Y22]),
            "Y22 should be high when A2 and B2 are both high"
        );
        assert!(
            low!(tr[Y23]),
            "Y23 should be low when A2 and B2 are both high"
        );
    }
}
