// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// Pin assignment for address pin A0.
    pub const A0: usize = 5;
    /// Pin assignment for address pin A1.
    pub const A1: usize = 7;
    /// Pin assignment for address pin A2.
    pub const A2: usize = 6;
    /// Pin assignment for address pin A3.
    pub const A3: usize = 12;
    /// Pin assignment for address pin A4.
    pub const A4: usize = 11;
    /// Pin assignment for address pin A5.
    pub const A5: usize = 10;
    /// Pin assignment for address pin A6.
    pub const A6: usize = 13;
    /// Pin assignment for address pin A7.
    pub const A7: usize = 9;

    /// Pin assignment for the data in pin.
    pub const D: usize = 2;
    /// Pin assignment for the data out pin.
    pub const Q: usize = 14;

    /// Pin assignment for the row address strobe pin.
    pub const RAS: usize = 4;
    /// Pin assignment for the column address strobe pin.
    pub const CAS: usize = 15;
    /// Pin assignment for the write enable pin.
    pub const WE: usize = 3;

    /// Pin assignment for the +5V power supply pin.
    pub const VCC: usize = 8;
    /// Pin assignment for the 0V (ground) power supply pin.
    pub const VSS: usize = 16;
    /// Pin assignment for the single no-contact pin.
    pub const NC: usize = 1;
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
    utils::pins_to_value,
};

use self::constants::*;

const PA_ADDRESS: [usize; 8] = [A0, A1, A2, A3, A4, A5, A6, A7];

/// An emulation of the 4164 64k x 1 bit dynamic RAM.
///
/// The 4164 is a basic DRAM chip that was used in a wide variety of home computers in the
/// 1980's: the Apple IIe, IIc, and 128k Macintosh; the Atari 800XL; the Commodore 64 and
/// 128; and the Radio Shack Color Computer 2. Later editions of the Apple IIc, Commodore
/// 64, Commodore 128, and COCO2 switched to the 41464.
///
/// This chip has a memory array of 65,536 bits, each associated with an individual memory
/// address. Therefore, to use a 4164 in an 8-bit computer, 8 chips would be required to
/// provide 64k of memory (128k Macintosh and Commodore 128 would therefore use 16 of these
/// chips). Each chip was used for a single bit in the target address; bit 0 would be stored
/// in the first 4164, bit 1 in the second 4164, and so on.
///
/// Since the chip has only 8 address pins, an address has to be split into two parts,
/// representing a row and a column (presenting the memory array as a physical 256-bit x
/// 256-bit array). These row and column addresses are provided to the chip sequentially;
/// the row address is put onto the address pins and  the active-low row address strobe pin
/// RAS is set low, then the column address is put onto the address pins and the active-low
/// column address strobe pin CAS is set low.
///
/// The chip has three basic modes of operation, controlled by the active-low write-enable
/// (WE) pin with some help from CAS. If WE is high, then the chip is in read mode after the
/// address is set. If WE is low, the mode depends on whether WE went low before the address
/// was set by putting CAS low; if CAS went low first, (meaning the chip was initially in
/// read mode), setting WE low will start read-modify-write mode, where the value at that
/// address is still available on the data-out pin (Q) even as the new value is set from the
/// data-in pin (D). If WE goes low before CAS, then read mode is never entered and write
/// mode is enabled instead. The value of D is still written to memory, but Q is
/// disconnected and no data is available there.
///
/// The Commodore 64 does not use read-modify-write mode. The WE pin is always set to its
/// proper level before the CAS pin goes low.
///
/// While WE and CAS control what is read from and/or written to the chip's memory, RAS is
/// not needed for anything other than setting the row address. Hence RAS can remain low
/// through multiple memory accesses, as long as its address is valid for all of them,
/// allowing reads and writes to happen within a single 256-address page of memory without
/// incurring the cost of resetting the row address. This doesn't happen in the C64; the
/// 6567 VIC cycles the RAS line once every clock cycle.
///
/// Unlike most other non-logic chips in the system, there is no dedicated chip-select pin.
/// The combination of RAS and CAS can be regarded as such a pin, and it is used that way in
/// the Commodore 64.
///
/// The chip comes in a 16-pin dual in-line package with the following pin assignments.
/// ```text
///         +---+--+---+
///      NC |1  +--+ 16| Vss
///       D |2       15| CAS
///      WE |3       14| Q
///     RAS |4       13| A6
///      A0 |5  4164 12| A3
///      A2 |6       11| A4
///      A1 |7       10| A5
///     Vcc |8        9| A7
///         +----------+
/// ```
/// These pin assignments are explained below.
///
/// | Pin | Name  | Description                                                            |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 1   | NC    | No connection. Not emulated.                                           |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 2   | D     | Data input. This pin's value is written to memory when write mode is   |
/// |     |       | entered.                                                               |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 3   | WE    | Active-low write enable. If this is low, memory is being written to.   |
/// |     |       | If it is high, memory is being read.                                   |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 4   | RAS   | Active-low row address strobe. When this goes low, the value of the    |
/// |     |       | address pins is stored as the row address for the internal 256x256     |
/// |     |       | memory array.                                                          |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 5   | A0    | Address pins. These 8 pins in conjunction with RAS and CAS allow the   |
/// | 6   | A2    | the addressing of 65,536 memory locations.                             |
/// | 7   | A1    |                                                                        |
/// | 9   | A7    |                                                                        |
/// | 10  | A5    |                                                                        |
/// | 11  | A4    |                                                                        |
/// | 12  | A3    |                                                                        |
/// | 13  | A6    |                                                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 8   | Vcc   | +5V power supply. Not emulated.                                        |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 14  | Q     | Data output. The value of the memory at the latched location appears   |
/// |     |       | on this pin when the CAS pin goes low in read mode.                    |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 15  | CAS   | Active-low column address strobe. When this goes low, the value of the |
/// |     |       | address pins is stored as the column address for the internal 256x256  |
/// |     |       | memory array, and the location is either read from or written to,      |
/// |     |       | depending on the value of WE.                                          |
/// | --- | ----- | ---------------------------------------------------------------------- |
/// | 16  | Vss   | 0V power supply (ground). Not emulated.                                |
///
/// In the Commodore 64, U9, U10, U11, U12, U21, U22, U23, and U24 are 4164s, one for each
/// of the 8 bits on the data bus.
pub struct Ic4164 {
    /// The pins of the 4164, along with a dummy pin (at index 0) to ensure that the vector
    /// index of the others matches the 1-based pin assignments.
    pins: RefVec<Pin>,

    /// Separate references to the A0-A7 pins in the `pins` vector.
    addr_pins: RefVec<Pin>,

    /// The place where the data is actually stored. The 4164 is 1-bit memory that is stored
    /// in a 256x256 matrix internally, but we don't have either u1 or u256 types (bools
    /// don't count; they actually take up much more than 1 bit of memory space). Instead we
    /// pack the bits into an array of 2048 u32s, which we then address through a function
    /// that resolves the row and column into an array index and an index to the bit inside
    /// the u32 value at that array index.
    memory: [u32; 2048],

    /// The latched row value taken from the pins when RAS transitions low. If no row has
    /// been latched (RAS hasn't yet gone low), this will be `None`.
    row: Option<u8>,

    /// The latched column value taken from the pins when CAS transitions low. If no column
    /// has been latched (CAS hasn't yet gone low), this will be `None`.
    col: Option<u8>,

    /// The latched data bit taken from the D pin. This is latched just before a write takes
    /// place and is done so that its value can replace the Q pin's value in RMW mode
    /// easily. If no data has been latched (either WE or CAS is not low), this will be
    /// `None`.
    data: Option<u8>,
}

impl Ic4164 {
    /// Creates a new 4164 64k x 1 dynamic RAM emulation and returns a shared, internally
    /// mutable reference to it.
    pub fn new() -> DeviceRef {
        // Address pins 0-7.
        let a0 = pin!(A0, "A0", Input);
        let a1 = pin!(A1, "A1", Input);
        let a2 = pin!(A2, "A2", Input);
        let a3 = pin!(A3, "A3", Input);
        let a4 = pin!(A4, "A4", Input);
        let a5 = pin!(A5, "A5", Input);
        let a6 = pin!(A6, "A6", Input);
        let a7 = pin!(A7, "A7", Input);

        // The data input pin. When the chip is in write or read-modify-write mode, the
        // value of this pin will be written to the appropriate bit in the memory array.
        let d = pin!(D, "D", Input);

        // The data output pin. This is active in read and read-modify-write mode, set to
        // the value of the bit at the address latched by RAS and CAS. In write mode, it is
        // hi-Z.
        let q = pin!(Q, "Q", Output);

        // The row address strobe. Setting this low latches the values of A0-A7, saving them
        // to be part of the address used to access the memory array.
        let ras = pin!(RAS, "RAS", Input);

        // The column address strobe. Setting this low latches A0-A7 into the second part of
        // the memory address. It also initiates read or write mode, depending on the value
        // of WE.
        let cas = pin!(CAS, "CAS", Input);

        // The write-enable pin. If this is high, the chip is in read mode; if it and CAS
        // are low, the chip is in either write or read-modify-write mode, depending on
        // which pin went low first.
        let we = pin!(WE, "WE", Input);

        // Power supply and no-contact pins. These are not emulated.
        let nc = pin!(NC, "NC", Unconnected);
        let vcc = pin!(VCC, "VCC", Unconnected);
        let vss = pin!(VSS, "VSS", Unconnected);

        let pins = pins![a0, a1, a2, a3, a4, a5, a6, a7, d, q, ras, cas, we, nc, vcc, vss];
        let addr_pins = RefVec::with_vec(
            IntoIterator::into_iter(PA_ADDRESS)
                .map(|pa| clone_ref!(pins[pa]))
                .collect::<Vec<PinRef>>(),
        );

        let device: DeviceRef = new_ref!(Ic4164 {
            pins,
            addr_pins,
            memory: [0; 2048],
            row: None,
            col: None,
            data: None,
        });

        float!(q);
        attach_to!(device, ras, cas, we);

        device
    }

    /// Reads the row and col and calculates the specific bit in the memory array to which
    /// this row/col combination refers. The first element of the return value is the index
    /// of the 32-bit number in the memory array where that bit resides; the second element
    /// is the index of the bit within that 32-bit number.
    fn resolve(&self) -> (usize, usize) {
        // Unless there's a bug in this program, this method should never be called while
        // either `self.row` or `self.col` are `None`. So we actually *want* it to panic if
        // `unwrap()` fails.
        let row = self.row.unwrap() as usize;
        let col = self.col.unwrap() as usize;

        let row_index = row << 3;
        let col_index = (col & 0b1110_0000) >> 5;
        let bit_index = col & 0b0001_1111;

        (row_index | col_index, bit_index)
    }

    /// Retrieves a single bit from the memory array and sets the level of the Q pin to the
    /// value of that bit.
    fn read(&self) {
        let (index, bit) = self.resolve();
        let value = (self.memory[index] & (1 << bit)) >> bit;
        set_level!(self.pins[Q], Some(value as f64))
    }

    /// Writes the value of the D pin to a single bit in the memory array. If the Q pin is
    /// also connected, the value is also sent to it; this happens only in RMW mode and
    /// keeps the input and output data pins synched. (This guaranteed sync means that the
    /// C64 can connect these two pins with a PC board trace, but the C64 doesn't use RMW
    /// mode.)
    fn write(&mut self) {
        let (index, bit) = self.resolve();
        if self.data.unwrap() == 1 {
            self.memory[index] |= 1 << bit;
        } else {
            self.memory[index] &= !(1 << bit);
        }
        if !floating!(self.pins[Q]) {
            set_level!(self.pins[Q], Some(self.data.unwrap() as f64));
        }
    }
}

impl Device for Ic4164 {
    fn pins(&self) -> RefVec<Pin> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChange) {
        match event {
            LevelChange(pin) if number!(pin) == RAS => {
                // Invoked when the RAS pin changes level. When it goes low, the current
                // states of the A0-A7 pins are latched. The address is released when the
                // RAS pin goes high.
                //
                // Since this is the only thing that RAS is used for, it can be left low for
                // multiple memory accesses if its bits of the address remain the same for
                // those accesses. This can speed up reads and writes within the same page
                // by reducing the amount of setup needed for those reads and writes. (This
                // does not happen in the C64.)
                if high!(pin) {
                    self.row = None;
                } else {
                    self.row = Some(pins_to_value(&self.addr_pins) as u8);
                }
            }
            LevelChange(pin) if number!(pin) == CAS => {
                // Invoked when the CAS pin changes level.
                //
                // When CAS goes low, the current states of the A0-A7 pins are latched in a
                // smiliar way to when RAS goes low. What else happens depends on whether
                // the WE pin is low. If it is, the chip goes into write mode and the value
                // on the D pin is saved to a memory location referred to by the latched row
                // and column values. If WE is not low, read mode is entered, and the value
                // in that memory location is put onto the Q pin. (Setting the WE pin low
                // after CAS goes low sets read-modify-write mode; the read that CAS
                // initiated is still valid.)
                //
                // When CAS goes high, the Q pin is disconnected and the latched column and
                // data (if there is one) values are cleared.
                if high!(pin) {
                    float!(self.pins[Q]);
                    self.col = None;
                    self.data = None;
                } else {
                    self.col = Some(pins_to_value(&self.addr_pins) as u8);
                    if high!(self.pins[WE]) {
                        self.read();
                    } else {
                        self.data = Some(if high!(self.pins[D]) { 1 } else { 0 });
                        self.write();
                    }
                }
            }
            LevelChange(pin) if number!(pin) == WE => {
                // Invoked when the WE pin changes level.
                //
                // When WE is high, read mode is enabled (though the actual read will not be
                // available until both RAS and CAS are set low, indicating that the address
                // of the read is valid). The internal latched input data value is cleared.
                //
                // When WE goes low, the write mode that is enabled depends on whether CAS
                // is already low. If it is, the chip must have been in read mode and now
                // moves into read-modify-write mode. The data value on the Q pin remains
                // valid, and the valus on the D pin is latched and stored at the
                // appropriate memory location.
                //
                // If CAS is still high when WE goes low, the Q pin is disconnected. Nothing
                // further happens until CAS goes low; at that point, the chip goes into
                // write mode (data is written to memory but nothing is available to be
                // read).
                if high!(pin) {
                    self.data = None;
                } else {
                    if high!(self.pins[CAS]) {
                        float!(self.pins[Q]);
                    } else {
                        self.data = Some(if high!(self.pins[D]) { 1 } else { 0 });
                        self.write();
                    }
                }
            }
            _ => {}
        }
    }

    fn debug_fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}, {:?}, {:?}", self.row, self.col, self.data)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        components::trace::{Trace, TraceRef},
        test_utils::{make_traces, value_to_traces},
    };

    use super::*;

    fn before_each() -> (DeviceRef, RefVec<Trace>, RefVec<Trace>) {
        let device = Ic4164::new();
        let tr = make_traces(&device);

        set!(tr[WE]);
        set!(tr[RAS]);
        set!(tr[CAS]);

        let addr_tr = RefVec::with_vec(
            IntoIterator::into_iter(PA_ADDRESS)
                .map(|p| clone_ref!(tr[p]))
                .collect::<Vec<TraceRef>>(),
        );

        (device, tr, addr_tr)
    }

    #[test]
    fn read_mode_enable_q() {
        let (_, tr, _) = before_each();

        clear!(tr[RAS]);
        clear!(tr[CAS]);
        // data at 0x0000, which will be 0 initially
        assert!(low!(tr[Q]), "Q should have data during read");

        set!(tr[CAS]);
        set!(tr[RAS]);
        assert!(floating!(tr[Q]), "Q should be disabled after read");
    }

    #[test]
    fn write_mode_disable_q() {
        let (_, tr, _) = before_each();

        clear!(tr[RAS]);
        clear!(tr[WE]);
        clear!(tr[CAS]);
        assert!(floating!(tr[Q]), "Q should be disabled during write");

        set!(tr[CAS]);
        set!(tr[WE]);
        set!(tr[RAS]);
        assert!(floating!(tr[Q]), "Q should be disabled after write");
    }

    #[test]
    fn rmw_mode_enable_q() {
        let (_, tr, _) = before_each();

        clear!(tr[D]);
        clear!(tr[RAS]);
        clear!(tr[CAS]);
        clear!(tr[WE]);
        assert!(low!(tr[Q]), "Q should be enabled during RMW");

        set!(tr[WE]);
        set!(tr[CAS]);
        set!(tr[RAS]);
        assert!(floating!(tr[Q]), "Q should be disabled after RMW");
    }

    #[test]
    fn read_write_one_bit() {
        let (_, tr, _) = before_each();

        // Write is happening at 0x0000, so we don't need to set addresses at all
        set!(tr[D]);
        clear!(tr[WE]);
        clear!(tr[RAS]);
        clear!(tr[CAS]);
        // 1 is written to address 0x0000 at this point
        set!(tr[CAS]);
        set!(tr[RAS]);
        set!(tr[WE]);

        clear!(tr[RAS]);
        clear!(tr[CAS]);
        let value = high!(tr[Q]);
        set!(tr[CAS]);
        set!(tr[RAS]);

        assert!(value, "Value 1 not written to address 0x0000");
    }

    #[test]
    fn rmw_one_bit() {
        let (_, tr, _) = before_each();

        // Write is happening at 0x0000, so we don't need to set addresses at all
        set!(tr[D]);
        clear!(tr[RAS]);
        clear!(tr[CAS]);
        // in read mode, Q should be 0 because no data has been written to 0x0000 yet
        assert!(
            low!(tr[Q]),
            "Value 0 not read from address 0x0000 in RMW mode"
        );
        // Lower WE to go into RMW
        clear!(tr[WE]);
        // 1 is written to address 0x0000 at this point
        set!(tr[CAS]);
        set!(tr[RAS]);
        set!(tr[WE]);

        clear!(tr[RAS]);
        clear!(tr[CAS]);
        let value = high!(tr[Q]);
        set!(tr[CAS]);
        set!(tr[RAS]);

        assert!(value, "Value 1 not written to address 0x0000");
    }

    fn bit_value(row: usize, col: usize) -> usize {
        let bit = col & 0b0001_1111;
        (row >> bit) & 1
    }

    // Regular read and write of each of the chip's 65,536 memory locations.
    #[test]
    fn read_write_full() {
        let (_, tr, addr_tr) = before_each();

        // Write all 65,536 locations with a bit based on its address
        for addr in 0..=0xffff {
            let row = (addr & 0xff00) >> 8;
            let col = addr & 0x00ff;

            // set the row address
            value_to_traces(row, &addr_tr);
            clear!(tr[RAS]);

            // set the column address
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);

            // write a bit to that address
            set_level!(tr[D], Some(bit_value(row, col) as f64));
            clear!(tr[WE]);

            set!(tr[RAS]);
            set!(tr[CAS]);
            set!(tr[WE]);
        }

        // Read all 65,536 locations and make sure they read what they should
        for addr in 0..=0xffff {
            let row = (addr & 0xff00) >> 8;
            let col = addr & 0x00ff;

            // set the row address
            value_to_traces(row, &addr_tr);
            clear!(tr[RAS]);

            // set the column address
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);

            let expected = bit_value(row, col) as f64;
            let actual = if high!(tr[Q]) { 1.0 } else { 0.0 };

            assert_eq!(
                actual, expected,
                "Incorrect bit value at address ${:02X}",
                addr
            );

            set!(tr[RAS]);
            set!(tr[CAS]);
        }
    }

    // Read and write of one page (256 memory locations). These are all of the locations
    // corresponding with a particular row of the memory matrix, and when accessing memory
    // in this way, the row can be latched with RAS once and be valid for all reads and
    // writes to that same page.
    #[test]
    fn read_write_page() {
        let (_, tr, addr_tr) = before_each();

        let row = 0x30; // arbitrary
        value_to_traces(row, &addr_tr);
        clear!(tr[RAS]);

        for col in 0..=0xff {
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);

            set_level!(tr[D], Some(bit_value(row, col) as f64));
            clear!(tr[WE]);

            set!(tr[CAS]);
            set!(tr[WE]);
        }

        for col in 0..=0xff {
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);

            let expected = bit_value(row, col) as f64;
            let actual = if high!(tr[Q]) { 1.0 } else { 0.0 };

            assert_eq!(
                actual, expected,
                "Incorrect bit value at column ${:02X}",
                col
            );

            set!(tr[CAS]);
        }

        set!(tr[RAS]);
    }

    // In RMW mode (CAS goes low before WE), a value written to D is immediately reflected
    // to output pin Q.
    #[test]
    fn read_write_rmw_q() {
        let (_, tr, addr_tr) = before_each();

        let row = 0x30; // arbitrary
        value_to_traces(row, &addr_tr);
        clear!(tr[RAS]);

        for col in 0..=0xff {
            clear!(tr[D]);
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);
            assert!(low!(tr[Q]), "Q should start low in read mode");

            set!(tr[D]);
            clear!(tr[WE]);
            assert!(high!(tr[Q]), "Q should change to reflect D in RMW mode");

            set!(tr[WE]);
            set!(tr[CAS]);
        }
        set!(tr[RAS]);
    }

    // In write mode (WE goes low before CAS), the written value is NOT reflected on output
    // pin Q, which is held in a high-Z state instead.
    #[test]
    fn read_write_no_rmw_q() {
        let (_, tr, addr_tr) = before_each();

        let row = 0x30; // arbitrary
        value_to_traces(row, &addr_tr);
        clear!(tr[RAS]);

        for col in 0..=0xff {
            clear!(tr[D]);
            clear!(tr[WE]);
            value_to_traces(col, &addr_tr);
            clear!(tr[CAS]);
            assert!(floating!(tr[Q]), "Q should not reflect D in write mode");

            set!(tr[WE]);
            set!(tr[CAS]);
        }
        set!(tr[RAS]);
    }
}
