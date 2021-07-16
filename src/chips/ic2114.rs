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
        pin::{Mode::*, Pin, PinRef},
    },
    ref_vec::RefVec,
    utils::{mode_to_pins, pins_to_value, value_to_pins},
};

use self::constants::*;

const PA_ADDRESS: [usize; 10] = [A0, A1, A2, A3, A4, A5, A6, A7, A8, A9];
const PA_DATA: [usize; 4] = [D0, D1, D2, D3];

pub struct Ic2114 {
    pins: RefVec<Pin>,
    addr_pins: RefVec<Pin>,
    data_pins: RefVec<Pin>,
    memory: [u8; 512],
}

impl Ic2114 {
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

    fn read(&self, addr: u16) -> u8 {
        let (index, shift) = resolve(addr);
        (self.memory[index] & (0xf << shift)) >> shift
    }

    fn write(&mut self, addr: u16, value: u8) {
        let (index, shift) = resolve(addr);
        let current = self.memory[index] & !(0x0f << shift);
        self.memory[index] = current | (value << shift);
    }
}

fn resolve(addr: u16) -> (usize, usize) {
    (addr as usize >> 1, (addr as usize & 0x01) * 4)
}

impl Device for Ic2114 {
    fn pins(&self) -> Vec<PinRef> {
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
