// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// Pin assignment for address pin A0.
    pub const A0: usize = 5;
    /// Pin assignment for address pin A1.
    pub const A1: usize = 6;
    /// Pin assignment for address pin A2.
    pub const A2: usize = 7;
    /// Pin assignment for address pin A3.
    pub const A3: usize = 4;
    /// Pin assignment for address pin A4.
    pub const A4: usize = 3;
    /// Pin assignment for address pin A5.
    pub const A5: usize = 2;
    /// Pin assignment for address pin A6.
    pub const A6: usize = 1;
    /// Pin assignment for address pin A7.
    pub const A7: usize = 17;
    /// Pin assignment for address pin A8.
    pub const A8: usize = 16;
    /// Pin assignment for address pin A9.
    pub const A9: usize = 15;

    /// Pin assignment for data pin D0.
    pub const D0: usize = 14;
    /// Pin assignment for data pin D1.
    pub const D1: usize = 13;
    /// Pin assignment for data pin D2.
    pub const D2: usize = 12;
    /// Pin assignment for data pin D3.
    pub const D3: usize = 11;

    /// Pin assignment for the chip select pin.
    pub const CS: usize = 8;
    /// Pin assignment for the write enable pin.
    pub const WE: usize = 10;

    /// Pin assignment for the +5V power supply pin.
    pub const VCC: usize = 18;
    /// Pin assignment for the ground pin.
    pub const GND: usize = 9;
}

use crate::{
    components::{
        device::{Device, DeviceRef, LevelChange},
        pin::{
            Mode::{Input, Output, Unconnected},
            Pin, PinRef,
        },
    },
    utils::{mode_to_pins, pins_to_value, value_to_pins},
    vectors::RefVec,
};

use self::constants::*;

const PA_ADDRESS: [usize; 10] = [A0, A1, A2, A3, A4, A5, A6, A7, A8, A9];
const PA_DATA: [usize; 4] = [D0, D1, D2, D3];

/// An emulation of the 2114 1k x 4 bit static RAM.
///
/// Static RAM differs from dynamic RAM (the RAM generally used for computer memory) in that
/// it doesn't require periodic refresh cycles in order to retain data. Since no reads or
/// writes have to wait for these refresh cycles, static RAM is considerably faster than
/// dynamic RAM.
///
/// However, it's also considerably more expensive. For this reason, static RAM is generally
/// only in use in particularly speed-sensitive applications and in relatively small
/// amounts. For instance, modern CPU on-board cache RAM is static. The Commodore 64 uses it
/// for color RAM, which is accessed by the VIC at a much higher speed than the DRAM is
/// accessed by the CPU.
///
/// The 2114 has 1024 addressable locations that hold 4 bits each. Since the Commodore 64
/// has a fixed palette of 16 colors, 4 bits is all it needs. Therefore a single 2114 could
/// store 1k of colors and it isn't necessary to use it with a second 2114 to store full
/// 8-bit bytes.
///
/// The timing of reads and writes is particularly simple. If the chip select pin CS is low,
/// the 4 bits stored at the location given on its address pins is put onto the 4 data pins.
/// If the write enable pin WE is also low, then the value on the 4 data pins is stored at
/// the location given on its address pins. The CS pin can stay low for several cycles of
/// reads and writes; it does not require CS to return to high to start the next cycle.
///
/// The downside of this simple scheme is that care has to be taken to avoid unwanted
/// writes. Address changes should not take place while both CS and WE are low; since
/// address lines do not change simultaneously, changing addresses while both pins are low
/// can and will cause data to be written to multiple addresses, potentially overwriting
/// legitimate data. This is naturally emulated here for the same reason: the chip responds
/// to address line changes, and those changes do not happen simultaneously.
///
/// Aside from the active-low CS and WE pins, this simple memory device only has the
/// necessary address pins to address 1k of memory and the four necessary bidirectional data
/// pins. It's packages in an 18-pin dual-inline package with the following pin assignments.
/// ```text
///         +---+--+---+
///      A6 |1  +--+ 18| Vcc
///      A5 |2       17| A7
///      A4 |3       16| A8
///      A3 |4       15| A9
///      A0 |5  2114 14| D0
///      A1 |6       13| D1
///      A2 |7       12| D2
///      CS |8       11| D3
///     GND |9       10| WE
///         +----------+
/// ```
/// These pin assignments are explained below.
///
/// | Pin | Name  | Description                                                            |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 1   | A6    | Address pins. These 10 pins can address 1024 memory locations.         |
/// | 2   | A5    |                                                                        |
/// | 3   | A4    |                                                                        |
/// | 4   | A3    |                                                                        |
/// | 5   | A0    |                                                                        |
/// | 6   | A1    |                                                                        |
/// | 7   | A2    |                                                                        |
/// | 15  | A9    |                                                                        |
/// | 16  | A8    |                                                                        |
/// | 17  | A7    |                                                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 8   | CS    | Active-low chip select pin. Reading and writing can only be done when  |
/// |     |       | this pin is low.                                                       |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 9   | GND   | Electrical ground. Not emulated.                                       |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 10  | WE    | Active-low write enable pin. This controls whether the chip is being   |
/// |     |       | read from (high) or written to (low).                                  |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 11  | D3    | Data pins. Data to be written to memory must be on these pins, and     |
/// | 12  | D2    | data read from memory will appear on these pins.                       |
/// | 13  | D1    |                                                                        |
/// | 14  | D0    |                                                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 18  | Vcc   | +5V power supply. Not emulated.                                        |
///
/// In the Commodore 64, U6 is a 2114. As explained above, it was used strictly as RAM for
/// storing graphics colors.
pub struct Ic2114 {
    /// The pins of the 2114, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,

    /// Separate references to the A0-A9 pins in the `pins` vector.
    addr_pins: RefVec<Pin>,

    /// Separate references to the D0-D3 pins in the `pins` vector.
    data_pins: RefVec<Pin>,

    /// The place where the data is actually stored. The 2114 is 4-bit memory, and there is
    /// not a u4 type in Rust, so we use a u8 along with an address resolution function.
    /// (Memory is cheap and there isn't a practical reason to just not use a [u8; 1024] and
    /// ignore the high bits, but this feels a little better in an emulator that is supposed
    /// to mimic the hardware as closely as possible.)
    memory: [u8; 512],
}

impl Ic2114 {
    /// Creates a new 2114 1k x 4 static RAM emulation and returns a shared, internally
    /// mutable reference to it.
    pub fn new() -> DeviceRef {
        // Address pins A0-A9.
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

        // Data pins D0-D3. These are set to input mode initially, but they will swotch to
        // output mode during reads.
        let d0 = pin!(D0, "D0", Input);
        let d1 = pin!(D1, "D1", Input);
        let d2 = pin!(D2, "D2", Input);
        let d3 = pin!(D3, "D3", Input);

        // Chip select pin. Setting this to low is what begins a read or write cycle.
        let cs = pin!(CS, "CS", Input);

        // Write enable pin. If this is low when CS goes low, then the cycle is a write
        // cycle, otherwise it's a read cycle.
        let we = pin!(WE, "WE", Input);

        // Power supply and ground pins. These are not emulated.
        let vcc = pin!(VCC, "VCC", Unconnected);
        let gnd = pin!(GND, "GND", Unconnected);

        let pins = pins![a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, d0, d1, d2, d3, cs, we, vcc, gnd];
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
        let memory = [0; 512];

        let device: DeviceRef = new_ref!(Ic2114 {
            pins,
            addr_pins,
            data_pins,
            memory
        });
        attach_to!(device, a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, d0, d1, d2, d3, cs, we);

        device
    }

    /// Returns the contents of the memory at the given address.
    fn read(&self, addr: u16) -> u8 {
        let (index, shift) = resolve(addr);
        (self.memory[index] & (0xf << shift)) >> shift
    }

    /// Writes the provided value to the memory array at the given address.
    fn write(&mut self, addr: u16, value: u8) {
        let (index, shift) = resolve(addr);
        let current = self.memory[index] & !(0x0f << shift);
        self.memory[index] = current | (value << shift);
    }
}

/// Resolves an address to the actual indices within the memory array where that address
/// points. The returned tuple contains the index into the array, along with an index that
/// points to the low bit for the desired 4-bit value (this will always be either 0 or 4).
fn resolve(addr: u16) -> (usize, usize) {
    (addr as usize >> 1, (addr as usize & 0x01) * 4)
}

impl Device for Ic2114 {
    fn pins(&self) -> RefVec<Pin> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        macro_rules! read {
            () => {
                mode_to_pins(Output, &self.data_pins);
                let addr = pins_to_value(&self.addr_pins) as u16;
                let value = self.read(addr) as usize;
                value_to_pins(value, &self.data_pins);
            };
        }
        macro_rules! write {
            () => {
                mode_to_pins(Input, &self.data_pins);
                let addr = pins_to_value(&self.addr_pins) as u16;
                let value = pins_to_value(&self.data_pins) as u8;
                self.write(addr, value);
            };
        }

        match event {
            LevelChange(pin) if number!(pin) == CS => {
                if high!(pin) {
                    mode_to_pins(Input, &self.data_pins);
                } else if high!(self.pins[WE]) {
                    read!();
                } else {
                    write!();
                }
            }
            LevelChange(pin) if number!(pin) == WE => {
                if !high!(self.pins[CS]) {
                    if high!(pin) {
                        read!();
                    } else {
                        write!();
                    }
                }
            }
            LevelChange(pin) if PA_ADDRESS.contains(&number!(pin)) => {
                if !high!(self.pins[CS]) {
                    if high!(self.pins[WE]) {
                        read!();
                    } else {
                        write!();
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        components::trace::{Trace, TraceRef},
        test_utils::{make_traces, traces_to_value, value_to_traces},
    };

    use super::*;

    fn before_each() -> (DeviceRef, RefVec<Trace>, RefVec<Trace>, RefVec<Trace>) {
        let device = Ic2114::new();
        let tr = make_traces(&device);

        set!(tr[CS]);
        set!(tr[WE]);

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
    fn read_and_write() {
        let (_, tr, addr_tr, data_tr) = before_each();

        for addr in 0..0x400 {
            let value = addr & 0x0f;
            value_to_traces(addr, &addr_tr);
            value_to_traces(value, &data_tr);
            clear!(tr[WE]);
            clear!(tr[CS]);
            set!(tr[CS]);
            set!(tr[WE]);
        }

        for addr in 0..0x400 {
            let expected = addr & 0xf;
            value_to_traces(addr, &addr_tr);
            clear!(tr[CS]);
            let value = traces_to_value(&data_tr);
            set!(tr[CS]);

            assert_eq!(
                value, expected,
                "Incorrect value at address ${:03x}: expected ${:1x}, actual ${:1x}",
                addr, expected, value
            );
        }
    }
}
