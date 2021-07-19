// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// The pin assignment for address pin A0.
    pub const A0: usize = 8;
    /// The pin assignment for address pin A1.
    pub const A1: usize = 7;
    /// The pin assignment for address pin A2.
    pub const A2: usize = 6;
    /// The pin assignment for address pin A3.
    pub const A3: usize = 5;
    /// The pin assignment for address pin A4.
    pub const A4: usize = 4;
    /// The pin assignment for address pin A5.
    pub const A5: usize = 3;
    /// The pin assignment for address pin A6.
    pub const A6: usize = 2;
    /// The pin assignment for address pin A7.
    pub const A7: usize = 1;
    /// The pin assignment for address pin A8.
    pub const A8: usize = 23;
    /// The pin assignment for address pin A9.
    pub const A9: usize = 22;
    /// The pin assignment for address pin A10.
    pub const A10: usize = 18;
    /// The pin assignment for address pin A11.
    pub const A11: usize = 19;
    /// The pin assignment for address pin A11.
    pub const A12: usize = 21;

    /// The pin assignment for data pin D0.
    pub const D0: usize = 9;
    /// The pin assignment for data pin D1.
    pub const D1: usize = 10;
    /// The pin assignment for data pin D2.
    pub const D2: usize = 11;
    /// The pin assignment for data pin D3.
    pub const D3: usize = 13;
    /// The pin assignment for data pin D4.
    pub const D4: usize = 14;
    /// The pin assignment for data pin D5.
    pub const D5: usize = 15;
    /// The pin assignment for data pin D6.
    pub const D6: usize = 16;
    /// The pin assignment for data pin D7.
    pub const D7: usize = 17;

    /// The pin assignment for the chip select pin.
    pub const CS: usize = 20;

    /// The pin assignment for the +5V power supply pin.
    pub const VCC: usize = 24;
    /// The pin assignment for the ground pin.
    pub const GND: usize = 12;
}

use crate::{
    components::{
        device::{Device, DeviceRef, LevelChange},
        pin::{
            Mode::{Input, Output, Unconnected},
            Pin, PinRef,
        },
    },
    utils::{none_to_pins, pins_to_value, value_to_pins},
    vectors::RefVec,
};

use self::constants::*;

const PA_ADDRESS: [usize; 13] = [A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12];
const PA_DATA: [usize; 8] = [D0, D1, D2, D3, D4, D5, D6, D7];

/// An emulation of the 2364 8k x 8-bit ROM.
///
/// This, along with the similar 2332, is far and away the simplest memory chip in the
/// Commodore 64. With its full complement of address pins and full 8 data pins, there is no
/// need to use multiple chips or to multiplex addresses.
///
/// Timing of the read cycle (there is, of course, no write cycle in a read-only memory
/// chip) is based solely on the chip select pin `CS`. When this pin goes low, the chip
/// reads its address pins and makes the value at that location available on its data pins.
///
/// The chip comes in a 24-pin dual in-line package with the following pin assignments.
/// ```text
///         +-----+--+-----+
///      A7 |1    +--+   24| Vcc
///      A6 |2           23| A8
///      A5 |3           22| A9
///      A4 |4           21| A12
///      A3 |5           20| CS
///      A2 |6           19| A10
///      A1 |7    2364   18| A11
///      A0 |8           17| D7
///      D0 |9           16| D6
///      D1 |10          15| D5
///      D2 |11          14| D4
///     GND |12          13| D3
///         +--------------+
/// ```
/// These pin assignments are explained below.
///
/// | Pin | Name  | Description                                                            |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 1   | A7    | Address pins. These 13 pins can address 8192 memory locations.         |
/// | 2   | A6    |                                                                        |
/// | 3   | A5    |                                                                        |
/// | 4   | A4    |                                                                        |
/// | 5   | A3    |                                                                        |
/// | 6   | A2    |                                                                        |
/// | 7   | A1    |                                                                        |
/// | 8   | A0    |                                                                        |
/// | 18  | A11   |                                                                        |
/// | 19  | A10   |                                                                        |
/// | 21  | A12   |                                                                        |
/// | 22  | A9    |                                                                        |
/// | 23  | A8    |                                                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 9   | D0    | Data pins. Data being read from memory will appear on these pins.      |
/// | 10  | D1    |                                                                        |
/// | 11  | D2    |                                                                        |
/// | 13  | D3    |                                                                        |
/// | 14  | D4    |                                                                        |
/// | 15  | D5    |                                                                        |
/// | 16  | D6    |                                                                        |
/// | 17  | D7    |                                                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 12  | GND   | Electrical ground. Not emulated.                                       |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 20  | CS    | Active-low chip select pin. Reading memory can only be done while this |
/// |     |       | pin is low.                                                            |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 24  | Vcc   | +5V power supply. Not emulated.                                        |
///
/// In the Commodore 64, U3 and U4 are both 2364A's (a variant with slightly faster data
/// access). U3 stores the BASIC interpreter and U4 stores the kernal.
pub struct Ic2364 {
    /// The pins of the 2364, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,

    /// Separate references to the A0-A12 pins in the `pins` vector.
    addr_pins: RefVec<Pin>,

    /// Separate references to the D0-D7 pins in the `pins` vector.
    data_pins: RefVec<Pin>,

    /// The array in which the chip's memory is actually stored. This is set at creation
    /// time and cannot afterwards be changed.
    memory: [u8; 8192],
}

impl Ic2364 {
    /// Creates a new 2364 8k x 8 ROM emulation and returns a shared, internally mutable
    /// reference to it. The parameter is a reference to a 8k-length array that has the
    /// contents of the ROM's memory; these ROMs are found in the crate::roms module.
    pub fn new(bytes: &[u8; 8192]) -> DeviceRef {
        // Address pins A0-A12
        let a0 = pin!(A0, "A0", Input);
        let a1 = pin!(A1, "A1", Input);
        let a2 = pin!(A2, "A2", Input);
        let a3 = pin!(A3, "A3", Input);
        let a4 = pin!(A4, "A4", Input);
        let a5 = pin!(A5, "A5", Input);
        let a6 = pin!(A6, "A6", Input);
        let a7 = pin!(A7, "A7", Input);
        let a8 = pin!(A8, "A8", Input);
        let a9 = pin!(A9, "A9", Input);
        let a10 = pin!(A10, "A10", Input);
        let a11 = pin!(A11, "A11", Input);
        let a12 = pin!(A12, "A12", Input);

        // Data pins D0-D7. Since this is read-only memory, and unlike other chips that use
        // data pins, these pins will never change to input mode.
        let d0 = pin!(D0, "D0", Output);
        let d1 = pin!(D1, "D1", Output);
        let d2 = pin!(D2, "D2", Output);
        let d3 = pin!(D3, "D3", Output);
        let d4 = pin!(D4, "D4", Output);
        let d5 = pin!(D5, "D5", Output);
        let d6 = pin!(D6, "D6", Output);
        let d7 = pin!(D7, "D7", Output);

        // Chip select pin. When this goes low, a read cycle is executed based on the
        // address on pins A0-A12. When they're high, the data pins are put into hi-Z.
        let cs = pin!(CS, "CS", Input);

        // Power supply and ground pins. These are not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let pins = pins![
            a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, d0, d1, d2, d3, d4, d5, d6, d7,
            cs, vcc, gnd
        ];
        let addr_pins = RefVec::with_vec(
            IntoIterator::into_iter(PA_ADDRESS)
                .map(|pa| clone_ref!(pins[pa]))
                .collect::<Vec<PinRef>>(),
        );
        let data_pins = RefVec::with_vec(
            IntoIterator::into_iter(PA_DATA)
                .map(|pa| clone_ref!(pins[pa]))
                .collect::<Vec<PinRef>>(),
        );
        let memory = bytes.clone();

        let device: DeviceRef = new_ref!(Ic2364 {
            pins,
            addr_pins,
            data_pins,
            memory,
        });

        attach_to!(device, cs);

        device
    }
}

impl Device for Ic2364 {
    fn pins(&self) -> RefVec<Pin> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        match event {
            LevelChange(pin) => {
                if low!(pin) {
                    let value = self.memory[pins_to_value(&self.addr_pins)];
                    value_to_pins(value as usize, &self.data_pins);
                } else {
                    none_to_pins(&self.data_pins);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        components::trace::{Trace, TraceRef},
        roms::{ROM_BASIC, ROM_KERNAL},
        test_utils::{make_traces, traces_to_value, value_to_traces},
    };

    use super::*;

    fn before_each(bytes: &[u8; 8192]) -> (DeviceRef, RefVec<Trace>, RefVec<Trace>, RefVec<Trace>) {
        let device = Ic2364::new(bytes);
        let tr = make_traces(&device);

        set!(tr[CS]);

        let addr_tr = RefVec::with_vec(
            IntoIterator::into_iter(PA_ADDRESS)
                .map(|p| clone_ref!(tr[p]))
                .collect::<Vec<TraceRef>>(),
        );
        let data_tr = RefVec::with_vec(
            IntoIterator::into_iter(PA_DATA)
                .map(|p| clone_ref!(tr[p]))
                .collect::<Vec<TraceRef>>(),
        );

        (device, tr, addr_tr, data_tr)
    }

    #[test]
    fn read_full_basic() {
        let (_, tr, addr_tr, data_tr) = before_each(&ROM_BASIC);

        for addr in 0..=0x1fff {
            value_to_traces(addr, &addr_tr);
            clear!(tr[CS]);
            let value = traces_to_value(&data_tr);
            set!(tr[CS]);

            assert_eq!(
                value as u8, ROM_BASIC[addr],
                "Incorrect value at address ${:04X}: expected ${:X}, actual ${:X}",
                addr, ROM_BASIC[addr], value
            );
        }
    }

    #[test]
    fn read_full_kernal() {
        let (_, tr, addr_tr, data_tr) = before_each(&ROM_KERNAL);

        for addr in 0..=0x1fff {
            value_to_traces(addr, &addr_tr);
            clear!(tr[CS]);
            let value = traces_to_value(&data_tr);
            set!(tr[CS]);

            assert_eq!(
                value as u8, ROM_KERNAL[addr],
                "Incorrect value at address ${:04X}: expected ${:X}, actual ${:X}",
                addr, ROM_KERNAL[addr], value
            );
        }
    }
}
