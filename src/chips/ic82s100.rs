// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod constants {
    /// Pin assignment for input pin 0.
    pub const I0: usize = 9;
    /// Pin assignment for input pin 1.
    pub const I1: usize = 8;
    /// Pin assignment for input pin 2.
    pub const I2: usize = 7;
    /// Pin assignment for input pin 3.
    pub const I3: usize = 6;
    /// Pin assignment for input pin 4.
    pub const I4: usize = 5;
    /// Pin assignment for input pin 5.
    pub const I5: usize = 4;
    /// Pin assignment for input pin 6.
    pub const I6: usize = 3;
    /// Pin assignment for input pin 7.
    pub const I7: usize = 2;
    /// Pin assignment for input pin 8.
    pub const I8: usize = 27;
    /// Pin assignment for input pin 9.
    pub const I9: usize = 26;
    /// Pin assignment for input pin 10.
    pub const I10: usize = 25;
    /// Pin assignment for input pin 11.
    pub const I11: usize = 24;
    /// Pin assignment for input pin 12.
    pub const I12: usize = 23;
    /// Pin assignment for input pin 13.
    pub const I13: usize = 22;
    /// Pin assignment for input pin 14.
    pub const I14: usize = 21;
    /// Pin assignment for input pin 15.
    pub const I15: usize = 20;

    /// Pin assignment for output pin 0.
    pub const F0: usize = 18;
    /// Pin assignment for output pin 1.
    pub const F1: usize = 17;
    /// Pin assignment for output pin 2.
    pub const F2: usize = 16;
    /// Pin assignment for output pin 3.
    pub const F3: usize = 15;
    /// Pin assignment for output pin 4.
    pub const F4: usize = 13;
    /// Pin assignment for output pin 5.
    pub const F5: usize = 12;
    /// Pin assignment for output pin 6.
    pub const F6: usize = 11;
    /// Pin assignment for output pin 7.
    pub const F7: usize = 10;

    /// Pin assignment for the output enable pin.
    pub const OE: usize = 19;

    /// Pin assignment for the field programming pin.
    pub const FE: usize = 1;

    /// Pin assignment for the +5V power supply pin.
    pub const VCC: usize = 28;
    /// Pin assignment for the ground pin.
    pub const VSS: usize = 14;

    // These are alternate names for the input (I) and output (F) pins, matching purpose of
    // each pin in the Commodore 64. They can be used to access the same pins with a
    // different naming convention. For example, the I0 pin, which accepts the CAS signal
    // from the VIC, can be accessed regularly with `device.pins()[I0]` or
    // `device.pins()[9]`. With these constants, if so desired, it can also be accessed as
    // `device.pins()[CAS]`.
    //
    // Note that one of the regular names is I0 and that one of the alternate names is IO.
    // It's probably best to stick to using one set or the other and not mixing them.

    /// Alternate pin assignment for input pin 0.
    pub const CAS: usize = I0;
    /// Alternate pin assignment for input pin 1.
    pub const LORAM: usize = I1;
    /// Alternate pin assignment for input pin 2.
    pub const HIRAM: usize = I2;
    /// Alternate pin assignment for input pin 3.
    pub const CHAREN: usize = I3;
    /// Alternate pin assignment for input pin 4.
    pub const VA14: usize = I4;
    /// Alternate pin assignment for input pin 5.
    pub const A15: usize = I5;
    /// Alternate pin assignment for input pin 6.
    pub const A14: usize = I6;
    /// Alternate pin assignment for input pin 7.
    pub const A13: usize = I7;
    /// Alternate pin assignment for input pin 8.
    pub const A12: usize = I8;
    /// Alternate pin assignment for input pin 9.
    pub const BA: usize = I9;
    /// Alternate pin assignment for input pin 10.
    pub const AEC: usize = I10;
    /// Alternate pin assignment for input pin 11.
    pub const R_W: usize = I11;
    /// Alternate pin assignment for input pin 12.
    pub const EXROM: usize = I12;
    /// Alternate pin assignment for input pin 13.
    pub const GAME: usize = I13;
    /// Alternate pin assignment for input pin 14.
    pub const VA13: usize = I14;
    /// Alternate pin assignment for input pin 15.
    pub const VA12: usize = I15;

    /// Alternate pin assignment for output pin 0.
    pub const CASRAM: usize = F0;
    /// Alternate pin assignment for output pin 1.
    pub const BASIC: usize = F1;
    /// Alternate pin assignment for output pin 2.
    pub const KERNAL: usize = F2;
    /// Alternate pin assignment for output pin 3.
    pub const CHAROM: usize = F3;
    /// Alternate pin assignment for output pin 4.
    pub const GR_W: usize = F4;
    /// Alternate pin assignment for output pin 5.
    pub const IO: usize = F5;
    /// Alternate pin assignment for output pin 6.
    pub const ROML: usize = F6;
    /// Alternate pin assignment for output pin 7.
    pub const ROMH: usize = F7;
}

use crate::components::{
    device::{Device, DeviceRef, LevelChangeEvent},
    pin::{
        Mode::{Input, Output, Unconnected},
        PinRef,
    },
};

use self::constants::*;

/// An emulation of the 82S100 Programmable Logic Array, as it was programmed for early
/// Commodore 64s.
///
/// The 82S100 is a programmable logic chip made by Signetics and is regarded as the very
/// first programmable logic device, first released in 1975. It took 16 inputs and could
/// form up to 48 product terms (P-terms) by logically ANDind and NOTing selections of the
/// 16 inputs. These P-terms were then fed to another array that would logically OR them
/// selectively, producing up to 8 sum terms (S-terms) that were ultimately sent to the
/// output pins (possibly after NOTing them first). The programming for these P- and S-terms
/// could be done in the field in a similar manner to that of a PROM (programmable read-only
/// memory), and Commodore would program them as a part of production of the C64.
///
/// A single 82S100 could therefore serve the same function as a large number of 7400-series
/// logic devices, for instance. It was used in a vast array of machines, including other
/// Commodore computers and disk drives and those from several other companies. The
/// differences between the 82S100's used in each would be the programming of the logic
/// arrays.
///
/// CBM eventually created their own hard-coded versions of the 82S100, which were faster
/// and were cheaper for CBM to produce than it was for them to buy and program 82S100's.
/// The schematic from which this emulation is being produced is an early one, from July,
/// 1982, and at that time the C64 was still using the Signetics device.
///
/// The input pins of the 82S100 were generically named I0-I15, and the output pins were
/// similarly named F0-F7. That has been maintained here, though constants are provided to
/// be able to use the more C64-centric names that reflect the pins' actual functions in
/// that computer.
///
/// This chip was tasked with providing the chip enable signals for the C64's RAM and color
/// RAM; BASIC, kernal, and character ROM; registers from the 6526 CIAs, 6567 VIC, and 6581
/// SID; and two banks of cartridge ROM and two banks of cartridge I/O memory. The 6510
/// processor, having a 16-bit address bus, could access 64k of memory, but all of this RAM
/// and ROM together added up to closer to 84k. Therefore the PLA used a combination of
/// inputs, including some of the address bus, the specialized 6510 I/O port bus, and
/// signals from the VIC and cartridge ROMs to determine which memory was trying to be
/// accessed at any given time, and then provide chip enable signals for that memory
/// (turning it on and turning other memory that could be at the same address off). It would
/// do this for every memory access, which would happen twice per cycle (half the cycle was
/// used by the CPU, half by the VIC). Thus bank switching actually happened at a frequency
/// of 2MHz, twice the CPU clock frequency, and the chip generated a fair bit of heat.
///
/// The purpose of the logic in the PLA is to take the current state of the system during a
/// memory access and produce a single output enabling the one device (memory chip,
/// processor chip with register, cartridge, etc.) that will handle that memory access. (If
/// the selected output is IO, that indicates that the I/O block of memory is being
/// accessed, but the PLA cannot determine *which* device is being accessed from this
/// information alone. When that happens, a separate demultiplexer uses A8-A11 to determine
/// which device it is.)
///
/// ### Input pin assignments
///
/// * I0: CAS. This is the control line from the VIC that enables access to RAM. Instead of
///     going directly to RAM, this line comes to the PLA and if RAM access is selected, the
///     CASRAM output is sent to the RAM chips.
/// * I1: LORAM. This line controls the memory block from $A000 - $BFFF. When high (normal),
///     the BASIC ROM is available in this memory block. When low, it's switched out for
///     RAM.
/// * I2: HIRAM. This line controls the memory area from $E000 - $FFFF. When high (normal),
///     the KERNAL ROM is available in this memory block. When low, it's switched out for
///     RAM.
/// * I3: CHAREN. This line partially controls the memory block from $D000 - $DFFF. When
///     high (normal), the I/O devices appear in this memory block. When low, the character
///     ROM appears here instead. This line can be overridden by other signals, including
///     those allowing this memory block to contain RAM instead.
/// * I4, I14-I15: VA14, VA13, and VA12. When the VIC is active, these are used to determine
///     which block of memory the VIC is trying to access. VA14 is active low because that's
///     the way it's generated by CIA2; it's inverted back into active high before being
///     used to access memory but not for use with the PLA.
/// * I5-I8: A15-A12. Used to determine which block of memory the CPU wishes to access while
///     the CPU is active.
/// * I9: BA. The VIC sets this "Bus Available" line high to indicate normal bus operation,
///     switching between VIC and CPU each cycle. The VIC sets it low when it intends to
///     take exclusive access of the address bus.
/// * I10: AEC. This inverse of the VIC's "Address Enable Control" signal indicates which of
///     the CPU (low) or VIC (high) is in control of the data bus.
/// * I11: R_W. A high signal means that memory is being read, while a low means memory is
///     being written. This makes a difference to what memory is being accessed. For
///     example, ROM cannot be written; if an attempt is made to write to a ROM address
///     while ROM is banked in, the write will happen to RAM instead.
/// * I12-I13: GAME and EXROM. These are used by cartridges to indicate the presence of
///     addressable memory within them. There are two for-sure states for these two signals:
///     if both are high (the normal state), there is no cartridge; and if EXROM is high and
///     GAME is low, there is an Ultimax cartridge. Further mapping to cartridge ROM depends
///     not only on these two signals but also on LORAM, HIRAM, and CHAREN` there is a nice
///     table at https://www.c64-wiki.com/wiki/Bank_Switching#Mode_Table.
///
/// ### Output pin assignments
///
/// Since the outputs are fed to chip select pins, which are universally active low, all of
/// these are programmed to be inverted. Thus, at last seven of them will be high at any
/// given time and at most one will be low.
///
/// * F0: CASRAM. This is the signal that ultimately enables RAM. The production of this
///     signal is very different from the others. For all other output, if one of their
///     terms is selected, that output will be selected. For CASRAM, its terms are combined
///     with the terms from all other outputs (except for GR_W), and if any of the terms are
///     selected, then CASRAM will be *de*selected. (Even if all of those other outputs are
///     deselected, CASRAM will be deselected if CAS is high or if certain addresses are
///     accessed while an Ultimax cartridge is plugged in. In these cases, no PLA outputs
///     will be selected.)
/// * F1: BASIC. Enables the BASIC ROM.
/// * F2: KERNAL. Enables the KERNAL ROM.
/// * F3: CHAROM. Enables the Character ROM.
/// * F4: GR_W. Indicates that the static color RAM is being written to. Note that this is
///     the only output that is not actually a chip enable signal; color RAM is possibly
///     enabled if the IO output is selected (see the Ic74139 for details).
/// * F5: IO. Indicates that one of the following are going to be enabled: CIA1 registers,
///     CIA2 registers, VIC registers, SID registers, color RAM, expansion port I/O from
///     address $DE00 - $DEFF, or expansion port I/O from address $DF00 - $DFFF. Which of
///     these is actually enabled is done by decoding A8-A11, which is done by the 74139.
/// * F6: ROML. Enables expansion port ROM from $8000 - $9FFF.
/// * F7: ROMH. Enables expansion port ROM from $E000 - $EFFF.
///
/// There is a ton of information about the internal workings of the C64's version of the
/// 82S100 in "The C64 PLA Dissected" at
/// http://skoe.de/docs/c64-dissected/pla/c64_pla_dissected_a4ds.pdf. This document was used
/// to derive all of the logic in this object and has a number of interesting stories
/// besides (if you find that sort of thing interesting).
///
/// Additionally, the 82S100 has an active-low chip enable pin CE which is not used in the
/// Commodore 64 (it is tied directly to ground and therefore is always low, so the chip is
/// always enabled). There is also an FE pin that was used for programming the chip in the
/// field; the emulated chip from the C64 doesn't use this as the chip was programmed during
/// manufacturing.
///
/// The chip comes in a 28-pin dual in-line package with the following pin assignments.
/// ```text
///         +-----+--+-----+
///      FE |1    +--+   28| VCC
///      I7 |2           27| I8
///      I6 |3           26| I9
///      I5 |4           25| I10
///      I4 |5           24| I11
///      I3 |6           23| I12
///      I2 |7           22| I13
///      I1 |8   82S100  21| I14
///      I0 |9           20| I15
///      F7 |10          19| CE
///      F6 |11          18| F0
///      F5 |12          17| F1
///      F4 |13          16| F2
///     VSS |14          15| F3
///         +--------------+
/// ```
/// The pin assignments are very straightforward and are described here.
///
/// | Pin | Name  | C64 Name | Description                                                 |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 1   | FE    |          | Field programming pin. Used to program a PLA in the field.  |
/// |     |       |          | This pin is left unconnected in normal use and is not       |
/// |     |       |          | emulated.                                                   |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 2   | I7    | A13      | Input pins. These are connected to traces in the C64 that   |
/// | 3   | I6    | A14      | are described by their C64 name. Each of these traces is    |
/// | 4   | I5    | A15      | instrumental in determining which chip should be enabled for|
/// | 5   | I4    | VA14     | the next read or write. Details of the purpose for each of  |
/// | 6   | I3    | CHAREN   | these are desribed above.                                   |
/// | 7   | I2    | HIRAM    |                                                             |
/// | 8   | I1    | LORAM    |                                                             |
/// | 9   | I0    | CAS      |                                                             |
/// | 20  | I15   | VA12     |                                                             |
/// | 21  | I14   | VA13     |                                                             |
/// | 22  | I13   | GAME     |                                                             |
/// | 23  | I12   | EXROM    |                                                             |
/// | 24  | I11   | R_W      |                                                             |
/// | 25  | I10   | AEC      |                                                             |
/// | 26  | I9    | BA       |                                                             |
/// | 27  | I8    | A12      |                                                             |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 10  | F7    | ROMH     | Output pins. Each of these is active-low and leads to the   |
/// | 11  | F6    | ROML     | chip select pins of the chip (or expansion port pin, in the |
/// | 12  | F5    | IO       | case of ROML and ROMH) described by its C64 name. Details   |
/// | 13  | F4    | GR_W     | about the purpose for each of these signals is described    |
/// | 15  | F3    | CHAROM   | above.                                                      |
/// | 16  | F2    | KERNAL   |                                                             |
/// | 17  | F1    | BASIC    |                                                             |
/// | 18  | F0    | CASRAM   |                                                             |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 14  | VSS   |          | Electrical ground. Not emulated.                            |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 19  | CE    |          | Active-low chip enable. Always low (enabled) in the C64.    |
/// | --- | ----- | -------- | ----------------------------------------------------------- |
/// | 28  | VCC   |          | +5V power supply. Not emulated.                             |
///
/// In the Commodore 64, U17 is an 82S100. As detailed extensively above, it was used to
/// decode signals to determine which chip would receive a particular read or write.
pub struct Ic82S100 {
    pins: Vec<PinRef>,
}

impl Ic82S100 {
    pub fn new() -> DeviceRef {
        // Input pins. In the 82S100, these were generically named I0 through I15, since
        // each pin could serve any function depending on the programming applied.
        let i0 = pin!(I0, "I0", Input);
        let i1 = pin!(I1, "I1", Input);
        let i2 = pin!(I2, "I2", Input);
        let i3 = pin!(I3, "I3", Input);
        let i4 = pin!(I4, "I4", Input);
        let i5 = pin!(I5, "I5", Input);
        let i6 = pin!(I6, "I6", Input);
        let i7 = pin!(I7, "I7", Input);
        let i8 = pin!(I8, "I8", Input);
        let i9 = pin!(I9, "I9", Input);
        let i10 = pin!(I10, "I10", Input);
        let i11 = pin!(I11, "I11", Input);
        let i12 = pin!(I12, "I12", Input);
        let i13 = pin!(I13, "I13", Input);
        let i14 = pin!(I14, "I14", Input);
        let i15 = pin!(I15, "I15", Input);

        // Output pins. Similar to the input pins, these were named generically on the 82S100.
        let f0 = pin!(F0, "F0", Output);
        let f1 = pin!(F1, "F1", Output);
        let f2 = pin!(F2, "F2", Output);
        let f3 = pin!(F3, "F3", Output);
        let f4 = pin!(F4, "F4", Output);
        let f5 = pin!(F5, "F5", Output);
        let f6 = pin!(F6, "F6", Output);
        let f7 = pin!(F7, "F7", Output);

        // Output enable, disables all outputs when set high.
        let oe = pin!(OE, "OE", Input);

        // Field programming pin, not used in mask programmed parts and not emulated.
        let fe = pin!(FE, "FE", Unconnected);

        // Power supply and ground pins, not emulated
        let vcc = pin!(VCC, "VCC", Unconnected);
        let vss = pin!(VSS, "VSS", Unconnected);

        let device: DeviceRef = new_ref!(Ic82S100 {
            pins: pins![
                i0, i1, i2, i3, i4, i5, i6, i7, i8, i9, i10, i11, i12, i13, i14, i15, f0, f1, f2,
                f3, f4, f5, f6, f7, oe, fe, vcc, vss
            ],
        });

        clear!(f0);
        set!(f1, f2, f3, f4, f5, f6, f7);

        attach!(i0, clone_ref!(device));
        attach!(i1, clone_ref!(device));
        attach!(i2, clone_ref!(device));
        attach!(i3, clone_ref!(device));
        attach!(i4, clone_ref!(device));
        attach!(i5, clone_ref!(device));
        attach!(i6, clone_ref!(device));
        attach!(i7, clone_ref!(device));
        attach!(i8, clone_ref!(device));
        attach!(i9, clone_ref!(device));
        attach!(i10, clone_ref!(device));
        attach!(i11, clone_ref!(device));
        attach!(i12, clone_ref!(device));
        attach!(i13, clone_ref!(device));
        attach!(i14, clone_ref!(device));
        attach!(i15, clone_ref!(device));
        attach!(oe, clone_ref!(device));

        device
    }
}

impl Device for Ic82S100 {
    fn pins(&self) -> Vec<PinRef> {
        self.pins.clone()
    }

    fn registers(&self) -> Vec<u8> {
        vec![]
    }

    fn update(&mut self, event: &LevelChangeEvent) {
        macro_rules! value_in {
            ($pin:expr, $target:expr, $level:expr) => {
                (if *$pin == $target {
                    *$level
                } else {
                    level!(self.pins[$target])
                })
                .unwrap_or_default()
                    >= 0.5
            };
        }
        macro_rules! value_out {
            ($value:expr, $target:expr) => {
                set_level!(
                    self.pins[$target],
                    if $value { Some(1.0) } else { Some(0.0) }
                )
            };
        }

        match event {
            LevelChangeEvent(p, _, level)
                if *p == OE && level.is_some() && level.unwrap() >= 0.5 =>
            {
                float!(
                    self.pins[F0],
                    self.pins[F1],
                    self.pins[F2],
                    self.pins[F3],
                    self.pins[F4],
                    self.pins[F5],
                    self.pins[F6],
                    self.pins[F7]
                );
            }
            LevelChangeEvent(p, _, level) => {
                // These are the product term equations programmed into the PLA for use in a
                // C64. The names for each signal reflect the names of the pins that those
                // signals come from, and while that is an excellent way to make long and
                // complex code succinct, it doesn't do much for the human reader. For that
                // reason, each term has a comment to describe in more human terms what is
                // happening with that piece of the algorithm.
                //
                // Each P-term below has a comment with three lines. The first line
                // describes the state of the three 6510 I/O port lines that are used for
                // bank switching (LORAM, HIRAM, and CHAREN). The second line is the memory
                // address that needs to be accessed to select that P-term (this is from
                // either the regular address bus when the CPU is active or the VIC address
                // bus when the VIC is active). The final line gives information about
                // whether the CPU or the VIC is active, whether the memory access is a read
                // or a write, and what type (if any) of cartridge must be plugged into the
                // expansion port (the cartridge informaion takes into account the values of
                // LORAM, HIRAM, and CHAREN already).
                //
                // If any piece of information is not given, its value doesn't matter to
                // that P-term. For example, in p0, the comment says that LORAM and HIRAM
                // must both be deselected. CHAREN isn't mentioned because whether it is
                // selected or not doesn't change whether that P-term is selected or not.
                //
                // Oftentimes, the reason for multiple terms for one output selection is the
                // limitation on what can be checked in a single logic term, given that no
                // ORs are possible in the production of P-terms. For example, it is very
                // common to see two terms that are identical except that one indicates "no
                // cartridge or 8k cartridge" while the other has "16k cartridge". These two
                // terms together really mean "anything but an Ultimax cartridge", but
                // there's no way to do that in a single term with only AND and NOT.
                //
                // This information comes from the excellent paper available at
                // skoe.de/docs/c64-dissected/pla/c64_pla_dissected_a4ds.pdf. If this sort
                // of thing interests you, there's no better place for information about the
                // C64 PLA.
                let i0 = value_in!(p, CAS, level);
                let i1 = value_in!(p, LORAM, level);
                let i2 = value_in!(p, HIRAM, level);
                let i3 = value_in!(p, CHAREN, level);
                let i4 = value_in!(p, VA14, level);
                let i5 = value_in!(p, A15, level);
                let i6 = value_in!(p, A14, level);
                let i7 = value_in!(p, A13, level);
                let i8 = value_in!(p, A12, level);
                let i9 = value_in!(p, BA, level);
                let i10 = value_in!(p, AEC, level);
                let i11 = value_in!(p, R_W, level);
                let i12 = value_in!(p, EXROM, level);
                let i13 = value_in!(p, GAME, level);
                let i14 = value_in!(p, VA13, level);
                let i15 = value_in!(p, VA12, level);

                // LORAM deselected, HIRAM deselected
                // $A000 - $BFFF
                // CPU active, Read, No cartridge or 8k cartridge
                let p0 = i1 & i2 & i5 & !i6 & i7 & !i10 & i11 & i13;

                // HIRAM deselected
                // $E000 - $FFFF
                // CPU active, Read, No cartridge or 8k cartridge
                let p1 = i2 & i5 & i6 & i7 & !i10 & i11 & i13;

                // HIRAM deselected
                // $E000 - $FFFF
                // CPU active, Read, 16k cartridge
                let p2 = i2 & i5 & i6 & i7 & !i10 & i11 & !i12 & !i13;

                // HIRAM deselected, CHAREN selected
                // $D000 - $DFFF
                // CPU active, Read, No cartridge or 8k cartridge
                let p3 = i2 & !i3 & i5 & i6 & !i7 & i8 & !i10 & i11 & i13;

                // LORAM deselected, CHAREN selected
                // $D000 - $DFFF
                // CPU active, Read, No cartridge or 8k cartridge
                let p4 = i1 & !i3 & i5 & i6 & !i7 & i8 & !i10 & i11 & i13;

                // HIRAM deselected, CHAREN selected
                // $D000 - $DFFF
                // CPU active, Read, 16k cartridge
                let p5 = i2 & !i3 & i5 & i6 & !i7 & i8 & !i10 & i11 & !i12 & !i13;

                //
                // $1000 - $1FFF or $9000 - $9FFF
                // VIC active, No cartridge or 8k cartridge
                let p6 = i4 & !i14 & i15 & i10 & i13;

                //
                // $1000 - $1FFF or $9000 - $9FFF
                // VIC active, 16k cartridge
                let p7 = i4 & !i14 & i15 & i10 & !i12 & !i13;

                // Unused. May be a relic from earlier design in C64 prototypes that never
                // got removed.
                // let p8 = i0 & i5 & i6 & !i7 & i8 & !i10 & !i11;

                // HIRAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Bus available, Read, No cartridge or 8k cartridge
                let p9 = i2 & i3 & i5 & i6 & !i7 & i8 & !i10 & i9 & i11 & i13;

                // HIRAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Write, No cartridge or 8k cartridge
                let p10 = i2 & i3 & i5 & i6 & !i7 & i8 & !i10 & !i11 & i13;

                // LORAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Bus available, Read, No cartridge or 8k cartridge
                let p11 = i1 & i3 & i5 & i6 & !i7 & i8 & !i10 & i9 & i11 & i13;

                // LORAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Write, No cartridge or 8k cartridge
                let p12 = i1 & i3 & i5 & i6 & !i7 & i8 & !i10 & !i11 & i13;

                // HIRAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Bus available, Read, 16k cartridge
                let p13 = i2 & i3 & i5 & i6 & !i7 & i8 & !i10 & i9 & i11 & !i12 & !i13;

                // HIRAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Write, 16k cartridge
                let p14 = i2 & i3 & i5 & i6 & !i7 & i8 & !i10 & !i11 & !i12 & !i13;

                // LORAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Bus available, Read, 16k cartridge
                let p15 = i1 & i3 & i5 & i6 & !i7 & i8 & !i10 & i9 & i11 & !i12 & !i13;

                // LORAM deselected, CHAREN deselected
                // $D000 - $DFFF
                // CPU active, Write, 16k cartridge
                let p16 = i1 & i3 & i5 & i6 & !i7 & i8 & !i10 & !i11 & !i12 & !i13;

                //
                // $D000 - $DFFF
                // CPU active, Bus available, Read, Ultimax cartridge
                let p17 = i5 & i6 & !i7 & i8 & !i10 & i9 & i11 & i12 & !i13;

                //
                // $D000 - $DFFF
                // CPU active, Write, Ultimax cartridge
                let p18 = i5 & i6 & !i7 & i8 & !i10 & !i11 & i12 & !i13;

                // LORAM deselected, HIRAM deselected
                // $8000 - $9FFF
                // CPU active, Read, 8k or 16k cartridge
                let p19 = i1 & i2 & i5 & !i6 & !i7 & !i10 & i11 & !i12;

                //
                // $8000 - $9FFF
                // CPU active, Ultimax cartridge
                let p20 = i5 & !i6 & !i7 & !i10 & i12 & !i13;

                // HIRAM deselected
                // $A000 - $BFFF
                // CPU active, Read, 16k cartridge
                let p21 = i2 & i5 & !i6 & i7 & !i10 & i11 & !i12 & !i13;

                //
                // $E000 - $EFFF
                // CPU active, Ultimax cartridge
                let p22 = i5 & i6 & i7 & !i10 & i12 & !i13;

                //
                // $3000 - $3FFF, $7000 - $7FFF, $B000 - $BFFF, or $E000 - $EFFF
                // VIC active, Ultimax cartridge
                let p23 = i14 & i15 & i10 & i12 & !i13;

                //
                // $1000 - $1FFF or $3000 - $3FFF
                // Ultimax cartridge
                let p24 = !i5 & !i6 & i8 & i12 & !i13;

                //
                // $2000 - $3FFF
                // Ultimax cartridge
                let p25 = !i5 & !i6 & i7 & i12 & !i13;

                //
                // $4000 - $7FFF
                // Ultimax cartridge
                let p26 = !i5 & i6 & i12 & !i13;

                //
                // $A000 - $BFFF
                // Ultimax cartridge
                let p27 = i5 & !i6 & i7 & i12 & !i13;

                //
                // $C000 - $CFFF
                // Ultimax cartridge
                let p28 = i5 & i6 & !i7 & !i8 & i12 & !i13;

                // Unused.
                // let p29 = !i1;

                // CAS deselected
                //
                //
                let p30 = i0;

                // CAS selected
                // $D000 - $DFFF
                // CPU access, Write
                let p31 = !i0 & i5 & i6 & !i7 & i8 & !i10 & !i11;

                // This is the sum-term (S-term) portion of the logic, where the P-terms
                // calculated above are logically ORed to poroduce a single output. This is
                // much simpler than P-term production because the P-terms handle everything
                // about chip selection, except that each chip may be the choice of several
                // different P-terms. That's the role of the S-term logic, to combine
                // P-terms to come up with single outputs.

                // Selects BASIC ROM.
                let s1 = p0;

                // Selects KERNAL ROM.
                let s2 = p1 | p2;

                // Selects Character ROM.
                let s3 = p3 | p4 | p5 | p6 | p7;

                // Selects I/O, color RAM, or processor registers.
                let s4 = p9 | p10 | p11 | p12 | p13 | p14 | p15 | p16 | p17 | p18;

                // Selects low cartridge ROM.
                let s5 = p19 | p20;

                // Selects high cartridge ROM.
                let s6 = p21 | p22 | p23;

                // Selects write mode for color RAM.
                let s7 = p31;

                // Deselects RAM. This is the only *de*selection, which is why it is the
                // only one not inverted in the state assignment below.
                let s0 = s1 | s2 | s3 | s4 | s5 | s6 | p24 | p25 | p26 | p27 | p28 | p30;

                value_out!(s0, CASRAM);
                value_out!(!s1, BASIC);
                value_out!(!s2, KERNAL);
                value_out!(!s3, CHAROM);
                value_out!(!s7, GR_W);
                value_out!(!s4, IO);
                value_out!(!s5, ROML);
                value_out!(!s6, ROMH);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        components::trace::TraceRef,
        test_utils::{make_traces, traces_to_value, value_to_traces},
    };

    use super::*;

    const INPUTS: [usize; 16] = [
        I0, I1, I2, I3, I4, I5, I6, I7, I8, I9, I10, I11, I12, I13, I14, I15,
    ];
    const OUTPUTS: [usize; 8] = [F0, F1, F2, F3, F4, F5, F6, F7];

    // This function was adapted from a C program that provides a 64k table of outputs for
    // PLA based on all of the possible inputs. The original is located at
    // http://www.zimmers.net/anonftp/pub/cbm/firmware/computers/c64/pla.c.
    fn get_expected(input: u16) -> u8 {
        let cas = input & (1 << 0) != 0;
        let loram = input & (1 << 1) != 0;
        let hiram = input & (1 << 2) != 0;
        let charen = input & (1 << 3) != 0;
        let va14 = input & (1 << 4) != 0;
        let a15 = input & (1 << 5) != 0;
        let a14 = input & (1 << 6) != 0;
        let a13 = input & (1 << 7) != 0;
        let a12 = input & (1 << 8) != 0;
        let ba = input & (1 << 9) != 0;
        let aec = input & (1 << 10) != 0;
        let r_w = input & (1 << 11) != 0;
        let exrom = input & (1 << 12) != 0;
        let game = input & (1 << 13) != 0;
        let va13 = input & (1 << 14) != 0;
        let va12 = input & (1 << 15) != 0;

        let f0 = (loram & hiram & a15 & !a14 & a13 & !aec & r_w & game)
            | (hiram & a15 & a14 & a13 & !aec & r_w & game)
            | (hiram & a15 & a14 & a13 & !aec & r_w & !exrom & !game)
            | (hiram & !charen & a15 & a14 & !a13 & a12 & !aec & r_w & game)
            | (loram & !charen & a15 & a14 & !a13 & a12 & !aec & r_w & game)
            | (hiram & !charen & a15 & a14 & !a13 & a12 & !aec & r_w & !exrom & !game)
            | (va14 & aec & game & !va13 & va12)
            | (va14 & aec & !exrom & !game & !va13 & va12)
            | (hiram & charen & a15 & a14 & !a13 & a12 & ba & !aec & r_w & game)
            | (hiram & charen & a15 & a14 & !a13 & a12 & !aec & !r_w & game)
            | (loram & charen & a15 & a14 & !a13 & a12 & ba & !aec & r_w & game)
            | (loram & charen & a15 & a14 & !a13 & a12 & !aec & !r_w & game)
            | (hiram & charen & a15 & a14 & !a13 & a12 & ba & !aec & r_w & !exrom & !game)
            | (hiram & charen & a15 & a14 & !a13 & a12 & !aec & !r_w & !exrom & !game)
            | (loram & charen & a15 & a14 & !a13 & a12 & ba & !aec & r_w & !exrom & !game)
            | (loram & charen & a15 & a14 & !a13 & a12 & !aec & !r_w & !exrom & !game)
            | (a15 & a14 & !a13 & a12 & ba & !aec & r_w & exrom & !game)
            | (a15 & a14 & !a13 & a12 & !aec & !r_w & exrom & !game)
            | (loram & hiram & a15 & !a14 & !a13 & !aec & r_w & !exrom)
            | (a15 & !a14 & !a13 & !aec & exrom & !game)
            | (hiram & a15 & !a14 & a13 & !aec & r_w & !exrom & !game)
            | (a15 & a14 & a13 & !aec & exrom & !game)
            | (aec & exrom & !game & va13 & va12)
            | (!a15 & !a14 & a12 & exrom & !game)
            | (!a15 & !a14 & a13 & exrom & !game)
            | (!a15 & a14 & exrom & !game)
            | (a15 & !a14 & a13 & exrom & !game)
            | (a15 & a14 & !a13 & !a12 & exrom & !game)
            | cas;
        let f1 = !loram | !hiram | !a15 | a14 | !a13 | aec | !r_w | !game;
        let f2 = (!hiram | !a15 | !a14 | !a13 | aec | !r_w | !game)
            & (!hiram | !a15 | !a14 | !a13 | aec | !r_w | exrom | game);
        let f3 = (!hiram | charen | !a15 | !a14 | a13 | !a12 | aec | !r_w | !game)
            & (!loram | charen | !a15 | !a14 | a13 | !a12 | aec | !r_w | !game)
            & (!hiram | charen | !a15 | !a14 | a13 | !a12 | aec | !r_w | exrom | game)
            & (!va14 | !aec | !game | va13 | !va12)
            & (!va14 | !aec | exrom | game | va13 | !va12);
        let f4 = cas | !a15 | !a14 | a13 | !a12 | aec | r_w;
        let f5 = (!hiram | !charen | !a15 | !a14 | a13 | !a12 | !ba | aec | !r_w | !game)
            & (!hiram | !charen | !a15 | !a14 | a13 | !a12 | aec | r_w | !game)
            & (!loram | !charen | !a15 | !a14 | a13 | !a12 | !ba | aec | !r_w | !game)
            & (!loram | !charen | !a15 | !a14 | a13 | !a12 | aec | r_w | !game)
            & (!hiram | !charen | !a15 | !a14 | a13 | !a12 | !ba | aec | !r_w | exrom | game)
            & (!hiram | !charen | !a15 | !a14 | a13 | !a12 | aec | r_w | exrom | game)
            & (!loram | !charen | !a15 | !a14 | a13 | !a12 | !ba | aec | !r_w | exrom | game)
            & (!loram | !charen | !a15 | !a14 | a13 | !a12 | aec | r_w | exrom | game)
            & (!a15 | !a14 | a13 | !a12 | !ba | aec | !r_w | !exrom | game)
            & (!a15 | !a14 | a13 | !a12 | aec | r_w | !exrom | game);
        let f6 = (!loram | !hiram | !a15 | a14 | a13 | aec | !r_w | exrom)
            & (!a15 | a14 | a13 | aec | !exrom | game);
        let f7 = (!hiram | !a15 | a14 | !a13 | aec | !r_w | exrom | game)
            & (!a15 | !a14 | !a13 | aec | !exrom | game)
            & (!aec | !exrom | game | !va13 | !va12);

        let mut output = 0;
        if f0 {
            output |= 1 << 0;
        }
        if f1 {
            output |= 1 << 1;
        }
        if f2 {
            output |= 1 << 2;
        }
        if f3 {
            output |= 1 << 3;
        }
        if f4 {
            output |= 1 << 4;
        }
        if f5 {
            output |= 1 << 5;
        }
        if f6 {
            output |= 1 << 6;
        }
        if f7 {
            output |= 1 << 7;
        }

        output
    }

    fn before_each() -> (DeviceRef, Vec<TraceRef>, Vec<TraceRef>, Vec<TraceRef>) {
        let device = Ic82S100::new();
        let tr = make_traces(clone_ref!(device));

        let trin = IntoIterator::into_iter(INPUTS)
            .map(|p| clone_ref!(tr[p]))
            .collect::<Vec<TraceRef>>();
        let trout = IntoIterator::into_iter(OUTPUTS)
            .map(|p| clone_ref!(tr[p]))
            .collect::<Vec<TraceRef>>();

        (device, tr, trin, trout)
    }

    #[test]
    fn disable_out_on_high_oe() {
        let (_, tr, _, _) = before_each();
        set!(tr[OE]);

        assert!(floating!(tr[F0]));
        assert!(floating!(tr[F1]));
        assert!(floating!(tr[F2]));
        assert!(floating!(tr[F3]));
        assert!(floating!(tr[F4]));
        assert!(floating!(tr[F5]));
        assert!(floating!(tr[F6]));
        assert!(floating!(tr[F7]));
    }

    #[test]
    fn logic_combinations() {
        let (_, tr, trin, trout) = before_each();
        clear!(tr[OE]);

        for value in 0..0xffff {
            let expected = get_expected(value);

            value_to_traces(value as usize, trin.clone());
            let actual = traces_to_value(trout.clone());

            assert_eq!(
                actual as usize, expected as usize,
                "Incorrect output for input {:016b}: expected {:08b}, actual {:08b}",
                value, expected, actual
            );
        }
    }
}
