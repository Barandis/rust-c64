#[derive(Copy, Clone, Debug)]
pub(super) enum Operation {
  ADC, // add with carry
  AND, // bitwise and with accumulator
  ASL, // arithmetic shift left
  BCC, // branch on carry clear
  BCS, // branch on carry set
  BEQ, // branch on equal (zero set)
  BIT, // bit test
  BMI, // branch on minus (negative set)
  BNE, // branch on not equal (zero clear)
  BPL, // branch on plus (negative clear)
  BRK, // break
  BVC, // branch on overflow clear
  BVS, // branch on overflow set
  CLC, // clear carry
  CLD, // clear decimal
  CLI, // clear interrupt disable
  CLV, // clear overflow
  CMP, // compare accumulator
  CPX, // compare X register
  CPY, // compare Y register
  DEC, // decrement memory
  DEX, // decrement X register
  DEY, // decrement Y registerr
  EOR, // exclusive or with accumulator
  INC, // increment memory
  INX, // increment X register
  INY, // increment Y register
  JMP, // jump
  JSR, // jump to subroutine
  LDA, // load accumulator
  LDX, // load X register
  LDY, // load Y register
  LSR, // logical shift right
  NOP, // no operation
  ORA, // bitwise or with accumulator
  PHA, // push accumulator
  PHP, // push processor status
  PLA, // pull accumulator
  PLP, // pull processor status
  ROL, // rotate left
  ROR, // rotate right
  RTI, // return from interrupt
  RTS, // return from subroutine
  SBC, // subtract with carry
  SEC, // set carry
  SED, // set decimal
  SEI, // set interrupt disable
  STA, // store accumulator
  STX, // store X register
  STY, // store Y register
  TAX, // transfer accumulator to X
  TAY, // transfer accumulator to Y
  TSX, // transfer stack pointer to X
  TXA, // transfer X to accumulator
  TXS, // transfer X to stack pointer
  TYA, // transfer Y to accumulator

  // Undocumented Opcodes
  // Different sources give these different mnemonics, the ones here are as
  // shown at http://www.oxyron.de/html/opcodes02.html
  AHX,
  ALR,
  ANC, // and with accumulator, set carry to bit 7
  ARR,
  AXS,
  DCP, // decrement at address, then compare with accumulator
  ISC, // increment at address, then subtract with carry from accumulator
  KIL, // crash processor
  LAS,
  LAX, // load accumulator and X
  RLA, // rotate left and store, then and with accumulator
  RRA, // rotate right and store, then add with carry with accumulator
  SAX, // and accumulator with X, then store
  SHX,
  SHY,
  SLO, // arithmetic shift left and store, then or with accumulator
  SRE, // logical shift right and store, then exclusive or with accumulator
  TAS,
  XAA,
}

#[derive(Copy, Clone, Debug)]
pub(super) enum AddressingMode {
  Implied,
  Accumulator,
  Immediate,
  ZeroPage,
  ZeroPageX,
  ZeroPageY,
  Relative,
  Absolute,
  AbsoluteX,
  AbsoluteY,
  Indirect,
  IndirectX,
  IndirectY,
}

use AddressingMode::*;
use Operation::*;

// Opcodes, modes, and timings are taken from 
// http://www.6502.org/tutorials/6502opcodes.html.
//
// Undocumented opcode information comes from
// http://www.oxyron.de/html/opcodes02.html. 
//
// The operation and addressing mode in each of these tuples should be self-
// explanatory. The u8 is the number of cycles that the instruction takes to
// execute, and the bool is whether or not an additional cycle is added if a
// page boundary is crossed.
//
// Branch instructions can also cross page boundaries; however, since the
// addressing mode of branch instructions is relative, the absolute address of
// the branch is not calculated at decoding time. Therefore branch instructions
// handle the addition of page-crossing cycles themselves (as well as the
// additional cycle that they take when the branch occurs versus when it
// doesn't).
pub(super) const OPCODES: [(Operation, AddressingMode, u8, bool); 256] = [
  // 0x00 - 0x0f
  (BRK, Implied, 7, false),
  (ORA, IndirectX, 6, false),
  (KIL, Implied, 0, false),
  (SLO, IndirectX, 8, false),
  (NOP, ZeroPage, 3, false),
  (ORA, ZeroPage, 3, false),
  (ASL, ZeroPage, 5, false),
  (SLO, ZeroPage, 5, false),
  (PHP, Implied, 3, false),
  (ORA, Immediate, 2, false),
  (ASL, Accumulator, 2, false),
  (ANC, Immediate, 2, false),
  (NOP, Absolute, 4, false),
  (ORA, Absolute, 4, false),
  (ASL, Absolute, 6, false),
  (SLO, Absolute, 6, false),
  // 0x10 - 0x1f
  (BPL, Relative, 2, false),
  (ORA, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (SLO, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (ORA, ZeroPageX, 4, false),
  (ASL, ZeroPageX, 6, false),
  (SLO, ZeroPageX, 6, false),
  (CLC, Implied, 2, false),
  (ORA, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (SLO, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (ORA, AbsoluteX, 4, true),
  (ASL, AbsoluteX, 7, false),
  (SLO, AbsoluteX, 7, false),
  // 0x20 - 0x2f
  (JSR, Absolute, 6, false),
  (AND, IndirectX, 6, false),
  (KIL, Implied, 0, false),
  (RLA, IndirectX, 8, false),
  (BIT, ZeroPage, 3, false),
  (AND, ZeroPage, 3, false),
  (ROL, ZeroPage, 5, false),
  (RLA, ZeroPage, 5, false),
  (PLP, Implied, 4, false),
  (AND, Immediate, 2, false),
  (ROL, Accumulator, 2, false),
  (ANC, Immediate, 2, false),
  (BIT, Absolute, 4, false),
  (AND, Absolute, 4, false),
  (ROL, Absolute, 6, false),
  (RLA, Absolute, 6, false),
  // 0x30 - 0x3f
  (BMI, Relative, 2, false),
  (AND, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (RLA, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (AND, ZeroPageX, 4, false),
  (ROL, ZeroPageX, 6, false),
  (RLA, ZeroPageX, 7, false),
  (SEC, Implied, 2, false),
  (AND, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (RLA, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (AND, AbsoluteX, 4, true),
  (ROL, AbsoluteX, 7, false),
  (RLA, AbsoluteX, 7, false),
  // 0x40 - 0x4
  (RTI, Implied, 6, false),
  (EOR, IndirectX, 6, false),
  (KIL, Implied, 0, false),
  (SRE, IndirectX, 8, false),
  (NOP, ZeroPage, 3, false),
  (EOR, ZeroPage, 3, false),
  (LSR, ZeroPage, 5, false),
  (SRE, ZeroPage, 5, false),
  (PHA, Implied, 3, false),
  (EOR, Immediate, 2, false),
  (LSR, Accumulator, 2, false),
  (ALR, Immediate, 2, false),
  (JMP, Absolute, 3, false),
  (EOR, Absolute, 4, false),
  (LSR, Absolute, 6, false),
  (SRE, Absolute, 6, false),
  // 0x50 - 0x5f
  (BVC, Relative, 2, false),
  (EOR, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (SRE, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (EOR, ZeroPageX, 4, false),
  (LSR, ZeroPageX, 6, false),
  (SRE, ZeroPageX, 6, false),
  (CLI, Implied, 2, false),
  (EOR, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (SRE, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (EOR, AbsoluteX, 4, true),
  (LSR, AbsoluteX, 7, false),
  (SRE, AbsoluteX, 7, false),
  // 0x60 - 0x6f
  (RTS, Implied, 6, false),
  (ADC, IndirectX, 6, false),
  (KIL, Implied, 0, false),
  (RRA, IndirectX, 8, false),
  (NOP, ZeroPage, 3, false),
  (ADC, ZeroPage, 3, false),
  (ROR, ZeroPage, 5, false),
  (RRA, ZeroPage, 5, false),
  (PLA, Implied, 4, false),
  (ADC, Immediate, 2, false),
  (ROR, Accumulator, 2, false),
  (ARR, Immediate, 2, false),
  (JMP, Indirect, 5, false),
  (ADC, Absolute, 4, false),
  (ROR, Absolute, 6, false),
  (RRA, Absolute, 6, false),
  // 0x70 - 0x7f
  (BVS, Relative, 2, false),
  (ADC, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (RRA, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (ADC, ZeroPageX, 4, false),
  (ROR, ZeroPageX, 6, false),
  (RRA, ZeroPageX, 6, false),
  (SEI, Implied, 2, false),
  (ADC, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (RRA, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (ADC, AbsoluteX, 4, true),
  (ROR, AbsoluteX, 7, false),
  (RRA, AbsoluteX, 7, false),
  // 0x80 - 0x8f
  (NOP, Immediate, 2, false),
  (STA, IndirectX, 6, false),
  (NOP, Immediate, 2, false),
  (SAX, IndirectX, 6, false),
  (STY, ZeroPage, 3, false),
  (STA, ZeroPage, 3, false),
  (STX, ZeroPage, 3, false),
  (SAX, ZeroPage, 3, false),
  (DEY, Implied, 2, false),
  (NOP, Immediate, 2, false),
  (TXA, Implied, 2, false),
  (XAA, Immediate, 2, false),
  (STY, Absolute, 4, false),
  (STA, Absolute, 4, false),
  (STX, Absolute, 4, false),
  (SAX, Absolute, 4, false),
  // 0x90 - 0x9f
  (BCC, Relative, 2, false),
  (STA, IndirectY, 6, false),
  (KIL, Implied, 0, false),
  (AHX, IndirectY, 6, false),
  (STY, ZeroPageX, 4, false),
  (STA, ZeroPageX, 4, false),
  (STX, ZeroPageY, 4, false),
  (SAX, ZeroPageY, 4, false),
  (TYA, Implied, 2, false),
  (STA, AbsoluteY, 5, false),
  (TXS, Implied, 2, false),
  (TAS, AbsoluteY, 5, false),
  (SHY, AbsoluteX, 5, false),
  (STA, AbsoluteX, 5, false),
  (SHX, AbsoluteY, 5, false),
  (AHX, AbsoluteY, 5, false),
  // 0xa0 - 0xaf
  (LDY, Immediate, 2, false),
  (LDA, IndirectX, 6, false),
  (LDX, Immediate, 2, false),
  (LAX, IndirectX, 6, false),
  (LDY, ZeroPage, 3, false),
  (LDA, ZeroPage, 3, false),
  (LDX, ZeroPage, 3, false),
  (LAX, ZeroPage, 3, false),
  (TAY, Implied, 2, false),
  (LDA, Immediate, 2, false),
  (TAX, Implied, 2, false),
  (LAX, Immediate, 2, false),
  (LDY, Absolute, 4, false),
  (LDA, Absolute, 4, false),
  (LDX, Absolute, 4, false),
  (LAX, Absolute, 4, false),
  // 0xb0 - 0xbf
  (BCS, Relative, 2, false),
  (LDA, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (LAX, IndirectY, 5, true),
  (LDY, ZeroPageX, 4, false),
  (LDA, ZeroPageX, 4, false),
  (LDX, ZeroPageY, 4, false),
  (LAX, ZeroPageY, 4, false),
  (CLV, Implied, 2, false),
  (LDA, AbsoluteY, 4, true),
  (TSX, Implied, 2, false),
  (LAS, AbsoluteY, 4, true),
  (LDY, AbsoluteX, 4, true),
  (LDA, AbsoluteX, 4, true),
  (LDX, AbsoluteY, 4, true),
  (LAX, AbsoluteY, 4, true),
  // 0xc0 - 0xcf
  (CPY, Immediate, 2, false),
  (CMP, IndirectX, 6, false),
  (NOP, Immediate, 2, false),
  (DCP, IndirectX, 8, false),
  (CPY, ZeroPage, 3, false),
  (CMP, ZeroPage, 3, false),
  (DEC, ZeroPage, 5, false),
  (DCP, ZeroPage, 5, false),
  (INY, Implied, 2, false),
  (CMP, Immediate, 2, false),
  (DEX, Implied, 2, false),
  (AXS, Immediate, 2, false),
  (CPY, Absolute, 4, false),
  (CMP, Absolute, 4, false),
  (DEC, Absolute, 6, false),
  (DCP, Absolute, 6, false),
  // 0xd0 - 0xdf
  (BNE, Relative, 2, false),
  (CMP, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (DCP, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (CMP, ZeroPageX, 4, false),
  (DEC, ZeroPageX, 6, false),
  (DCP, ZeroPageX, 6, false),
  (CLD, Implied, 2, false),
  (CMP, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (DCP, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (CMP, AbsoluteX, 4, true),
  (DEC, AbsoluteX, 7, false),
  (DCP, AbsoluteX, 7, false),
  // 0xe0 - 0xef
  (CPX, Immediate, 2, false),
  (SBC, IndirectX, 6, false),
  (NOP, Immediate, 2, false),
  (ISC, IndirectX, 8, false),
  (CPX, ZeroPage, 3, false),
  (SBC, ZeroPage, 3, false),
  (INC, ZeroPage, 5, false),
  (ISC, ZeroPage, 5, false),
  (INX, Implied, 2, false),
  (SBC, Immediate, 2, false),
  (NOP, Implied, 2, false), // This one is the documented NOP
  (SBC, Immediate, 2, false),
  (CPX, Absolute, 4, false),
  (SBC, Absolute, 4, false),
  (INC, Absolute, 6, false),
  (ISC, Absolute, 6, false),
  // 0xf0 - 0xff
  (BEQ, Relative, 2, false),
  (SBC, IndirectY, 5, true),
  (KIL, Implied, 0, false),
  (ISC, IndirectY, 8, false),
  (NOP, ZeroPageX, 4, false),
  (SBC, ZeroPageX, 4, false),
  (INC, ZeroPageX, 6, false),
  (ISC, ZeroPageX, 6, false),
  (SED, Implied, 2, false),
  (SBC, AbsoluteY, 4, true),
  (NOP, Implied, 2, false),
  (ISC, AbsoluteY, 7, false),
  (NOP, AbsoluteX, 4, true),
  (SBC, AbsoluteX, 4, true),
  (INC, AbsoluteX, 7, false),
  (ISC, AbsoluteX, 7, false),
];

#[derive(Copy, Clone, Debug)]
pub(super) struct Instruction {
  pub op: Operation,
  pub mode: AddressingMode,
  pub arg: u16,
  pub target: Option<u16>,
  pub cycles: u8,
  pub page_cycle: bool,
}
