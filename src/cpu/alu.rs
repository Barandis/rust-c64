use crate::common::get_bit;
use crate::cpu::instruction::AddressingMode;
use crate::cpu::instruction::AddressingMode::*;
use crate::cpu::instruction::Instruction;
use crate::cpu::instruction::Operation;
use crate::cpu::instruction::Operation::*;
use crate::cpu::instruction::OPCODES;
use crate::cpu::Cpu;
use crate::memory::Addressable;

const ADDRESS_BRK: u16 = 0xfffe;

pub(super) fn decode_instruction(cpu: &Cpu) -> (Instruction, u16, bool) {
  let ptr = cpu.pc;
  let opcode = cpu.read(ptr) as usize;
  let (op, mode, cycles, page_cycle) = OPCODES[opcode];
  let (arg, target, arg_length, paged) =
    decode_addressing_mode(cpu, mode, ptr.wrapping_add(1), generates_read(op));
  let instruction = Instruction {
    op,
    mode,
    arg,
    target,
    cycles,
    page_cycle,
  };
  (instruction, 1 + arg_length, paged)
}

fn decode_addressing_mode(
  cpu: &Cpu,
  mode: AddressingMode,
  ptr: u16,
  read: bool,
) -> (u16, Option<u16>, u16, bool) {
  match mode {
    Implied => (0, None, 0, false),
    Accumulator => (cpu.a as u16, None, 0, false),
    Immediate => {
      let value = if read { cpu.read(ptr) as u16 } else { 0 };
      (value, Some(ptr), 1, false)
    }
    ZeroPage => {
      let addr = cpu.read(ptr) as u16;
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 1, false)
    }
    ZeroPageX => {
      let addr = cpu.read(ptr).wrapping_add(cpu.x) as u16;
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 1, false)
    }
    ZeroPageY => {
      let addr = cpu.read(ptr).wrapping_add(cpu.y) as u16;
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 1, false)
    }
    Relative => {
      let value = if read { cpu.read(ptr) as u16 } else { 0 };
      (value, Some(ptr), 1, false)
    }
    Absolute => {
      let addr = cpu.read16(ptr);
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 2, false)
    }
    AbsoluteX => {
      let base = cpu.read16(ptr);
      let addr = base.wrapping_add(cpu.x as u16);
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 2, crossed_page_boundary(base, addr))
    }
    AbsoluteY => {
      let base = cpu.read16(ptr);
      let addr = base.wrapping_add(cpu.y as u16);
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 2, crossed_page_boundary(base, addr))
    }
    Indirect => {
      let addr = cpu.read16(ptr);
      let jmp = cpu.read_pagewrap16(addr);
      (0, Some(jmp), 2, false)
    }
    IndirectX => {
      let zp = cpu.read(ptr).wrapping_add(cpu.x);
      let addr = cpu.read_zero16(zp);
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 1, false)
    }
    IndirectY => {
      let zp = cpu.read(ptr);
      let base = cpu.read_zero16(zp);
      let addr = base.wrapping_add(cpu.y as u16);
      let value = if read { cpu.read(addr) as u16 } else { 0 };
      (value, Some(addr), 1, crossed_page_boundary(base, addr))
    }
  }
}

fn execute_targeted_instruction(cpu: &mut Cpu, target: Option<u16>, func: fn(&mut Cpu, u8) -> u8) {
  let r = cpu.read_target(target);
  let w = func(cpu, r);
  cpu.write_target(target, w);
}

pub(super) fn execute_instruction(cpu: &mut Cpu, i: Instruction) {
  let value = i.arg as u8;

  match i.op {
    ADC => execute_adc(cpu, value),
    AND => execute_and(cpu, value),
    ASL => {
      execute_targeted_instruction(cpu, i.target, execute_asl);
      cpu.update_acc_flags();
    }
    BCC => execute_bcc(cpu, value),
    BCS => execute_bcs(cpu, value),
    BEQ => execute_beq(cpu, value),
    BIT => execute_bit(cpu, value),
    BMI => execute_bmi(cpu, value),
    BNE => execute_bne(cpu, value),
    BPL => execute_bpl(cpu, value),
    BRK => execute_brk(cpu),
    BVC => execute_bvc(cpu, value),
    BVS => execute_bvs(cpu, value),
    CLC => execute_clc(cpu),
    CLD => execute_cld(cpu),
    CLI => execute_cli(cpu),
    CLV => execute_clv(cpu),
    CMP => execute_cmp(cpu, value),
    CPX => execute_cpx(cpu, value),
    CPY => execute_cpy(cpu, value),
    DEC => execute_targeted_instruction(cpu, i.target, execute_dec),
    DEX => execute_dex(cpu),
    DEY => execute_dey(cpu),
    EOR => execute_eor(cpu, value),
    INC => execute_targeted_instruction(cpu, i.target, execute_inc),
    INX => execute_inx(cpu),
    INY => execute_iny(cpu),
    JMP => execute_jmp(cpu, i.target.unwrap()),
    JSR => execute_jsr(cpu, i.target.unwrap()),
    LDA => execute_lda(cpu, value),
    LDX => execute_ldx(cpu, value),
    LDY => execute_ldy(cpu, value),
    LSR => {
      execute_targeted_instruction(cpu, i.target, execute_lsr);
      cpu.update_acc_flags();
    }
    NOP => execute_nop(),
    ORA => execute_ora(cpu, value),
    PHA => execute_pha(cpu),
    PHP => execute_php(cpu),
    PLA => execute_pla(cpu),
    PLP => execute_plp(cpu),
    ROL => execute_targeted_instruction(cpu, i.target, execute_rol),
    ROR => execute_targeted_instruction(cpu, i.target, execute_ror),
    RTI => execute_rti(cpu),
    RTS => execute_rts(cpu),
    SBC => execute_sbc(cpu, value),
    SEC => execute_sec(cpu),
    SED => execute_sed(cpu),
    SEI => execute_sei(cpu),
    STA => cpu.write_target(i.target, cpu.a),
    STX => cpu.write_target(i.target, cpu.x),
    STY => cpu.write_target(i.target, cpu.y),
    TAX => execute_tax(cpu),
    TAY => execute_tay(cpu),
    TSX => execute_tsx(cpu),
    TXA => execute_txa(cpu),
    TXS => execute_txs(cpu),
    TYA => execute_tya(cpu),
    // Undocumented instructions
    ANC => execute_anc(cpu, value),
    DCP => execute_targeted_instruction(cpu, i.target, execute_dcp),
    ISC => execute_targeted_instruction(cpu, i.target, execute_isc),
    KIL => panic!("KIL instruction encountered"),
    LAX => execute_lax(cpu, value),
    RLA => execute_targeted_instruction(cpu, i.target, execute_rla),
    RRA => execute_targeted_instruction(cpu, i.target, execute_rra),
    SAX => {
      let w = execute_sax(cpu);
      cpu.write_target(i.target, w)
    }
    SLO => execute_targeted_instruction(cpu, i.target, execute_slo),
    SRE => execute_targeted_instruction(cpu, i.target, execute_sre),
    _ => execute_unimplemented(i.op),
  }
}

// INSTRUCTIONS

fn binary_add(value1: u8, value2: u8, carry: bool) -> (u8, bool, bool) {
  let (n1, v1) = value1.overflowing_add(value2);
  let (n2, v2) = n1.overflowing_add(carry as u8);
  let sum = (value1 as i8 as i16) + (value2 as i8 as i16) + (carry as i16);
  (n2, v1 | v2, (sum < -128) || (sum > 127))
}

fn execute_binary_adc(cpu: &mut Cpu, value: u8) {
  let (result, carry, overflow) = binary_add(cpu.a, value, cpu.c);
  cpu.a = result;
  cpu.c = carry;
  cpu.v = overflow;
  cpu.update_acc_flags();
}

fn execute_decimal_adc(cpu: &mut Cpu, value: u8) {
  let mut al = (cpu.a & 0x0f) + (value & 0x0f) + (cpu.c as u8);
  if al >= 0x0a {
    al = ((al + 0x06) & 0x0f) + 0x10;
  }
  let mut au = ((cpu.a & 0xf0) as u16) + ((value & 0xf0) as u16) + (al as u16);
  if au >= 0xa0 {
    au += 0x60;
  }
  let ag = ((cpu.a & 0xf0) as i8 as i16) + ((value & 0xf0) as i8 as i16) + (al as i8 as i16);
  let (bin, _, _) = binary_add(cpu.a, value, cpu.c);

  // N, Z, and V flags are undocumented. They actually reflect the state of the
  // accumulator in binary, not decimal (this was changed in later versions of
  // the CPU). The C flag is documented and valid.
  cpu.n = ag & 0x80 > 0;
  cpu.v = (ag < -128) || (ag > 127);
  cpu.c = au > 0xff;
  cpu.z = bin == 0;

  cpu.a = (au & 0xff) as u8;
}

fn execute_adc(cpu: &mut Cpu, value: u8) {
  if cpu.d {
    execute_decimal_adc(cpu, value);
  } else {
    execute_binary_adc(cpu, value);
  }
}

fn execute_and(cpu: &mut Cpu, value: u8) {
  cpu.a &= value;
  cpu.update_acc_flags();
}

fn execute_asl(cpu: &mut Cpu, value: u8) -> u8 {
  let (num, _) = value.overflowing_mul(2);
  cpu.c = get_bit(value, 7);
  num
}

// Called when any branch instruction actually branches. When this happens, a
// single clock cycle is added to the instruction's timing. If a page boundary
// is crossed by the branch, another clock cycle is added.
fn execute_branch(cpu: &mut Cpu, value: u8) {
  let pc = cpu.pc;
  cpu.pc = cpu.pc.wrapping_add((value as i8) as u16);
  cpu.cycles_left += 1;
  if crossed_page_boundary(cpu.pc, pc) {
    cpu.cycles_left += 1;
  }
}

fn execute_bcc(cpu: &mut Cpu, value: u8) {
  if !cpu.c {
    execute_branch(cpu, value);
  }
}

fn execute_bcs(cpu: &mut Cpu, value: u8) {
  if cpu.c {
    execute_branch(cpu, value);
  }
}

fn execute_beq(cpu: &mut Cpu, value: u8) {
  if cpu.z {
    execute_branch(cpu, value);
  }
}

fn execute_bit(cpu: &mut Cpu, value: u8) {
  cpu.n = 0x80 & value > 0;
  cpu.v = 0x40 & value > 0;
  cpu.z = cpu.a & value == 0;
}

fn execute_bmi(cpu: &mut Cpu, value: u8) {
  if cpu.n {
    execute_branch(cpu, value);
  }
}

fn execute_bne(cpu: &mut Cpu, value: u8) {
  if !cpu.z {
    execute_branch(cpu, value);
  }
}

fn execute_bpl(cpu: &mut Cpu, value: u8) {
  if !cpu.n {
    execute_branch(cpu, value);
  }
}

fn execute_brk(cpu: &mut Cpu) {
  cpu.push_stack16(cpu.pc);
  cpu.push_stack(cpu.get_psr(true));
  cpu.pc = cpu.read16(ADDRESS_BRK);
}

fn execute_bvc(cpu: &mut Cpu, value: u8) {
  if !cpu.v {
    execute_branch(cpu, value);
  }
}

fn execute_bvs(cpu: &mut Cpu, value: u8) {
  if cpu.v {
    execute_branch(cpu, value);
  }
}

fn execute_clc(cpu: &mut Cpu) {
  cpu.c = false;
}

fn execute_cld(cpu: &mut Cpu) {
  cpu.d = false;
}

fn execute_cli(cpu: &mut Cpu) {
  cpu.i = false;
}

fn execute_clv(cpu: &mut Cpu) {
  cpu.v = false;
}

fn execute_compare(cpu: &mut Cpu, value1: u8, value2: u8) {
  let result = value1.wrapping_sub(value2);
  cpu.c = value1 >= value2;
  cpu.z = value1 == value2;
  cpu.n = result >= 128;
}

fn execute_cmp(cpu: &mut Cpu, value: u8) {
  execute_compare(cpu, cpu.a, value);
}

fn execute_cpx(cpu: &mut Cpu, value: u8) {
  execute_compare(cpu, cpu.x, value);
}

fn execute_cpy(cpu: &mut Cpu, value: u8) {
  execute_compare(cpu, cpu.y, value);
}

fn execute_dec(cpu: &mut Cpu, value: u8) -> u8 {
  let result = value.wrapping_sub(1);
  cpu.update_result_flags(result);
  result
}

fn execute_dex(cpu: &mut Cpu) {
  cpu.x = execute_dec(cpu, cpu.x);
}

fn execute_dey(cpu: &mut Cpu) {
  cpu.y = execute_dec(cpu, cpu.y);
}

fn execute_eor(cpu: &mut Cpu, value: u8) {
  cpu.a ^= value;
  cpu.update_acc_flags();
}

fn execute_inc(cpu: &mut Cpu, value: u8) -> u8 {
  let result = value.wrapping_add(1);
  cpu.update_result_flags(result);
  result
}

fn execute_inx(cpu: &mut Cpu) {
  cpu.x = execute_inc(cpu, cpu.x);
}

fn execute_iny(cpu: &mut Cpu) {
  cpu.y = execute_inc(cpu, cpu.y);
}

fn execute_jmp(cpu: &mut Cpu, ptr: u16) {
  cpu.pc = ptr;
}

fn execute_jsr(cpu: &mut Cpu, ptr: u16) {
  cpu.push_stack16(cpu.pc.wrapping_sub(1));
  cpu.pc = ptr;
}

fn execute_lda(cpu: &mut Cpu, value: u8) {
  cpu.a = value;
  cpu.update_acc_flags();
}

fn execute_ldx(cpu: &mut Cpu, value: u8) {
  cpu.x = value;
  cpu.update_result_flags(value);
}

fn execute_ldy(cpu: &mut Cpu, value: u8) {
  cpu.y = value;
  cpu.update_result_flags(value);
}

fn execute_lsr(cpu: &mut Cpu, value: u8) -> u8 {
  cpu.c = value & 0x01 > 0;
  let result = value.wrapping_shr(1);
  cpu.update_result_flags(result);
  result
}

fn execute_nop() {}

fn execute_ora(cpu: &mut Cpu, value: u8) {
  cpu.a |= value;
  cpu.update_acc_flags();
}

fn execute_pha(cpu: &mut Cpu) {
  cpu.push_stack(cpu.a);
}

fn execute_php(cpu: &mut Cpu) {
  cpu.push_stack(cpu.get_psr(true));
}

fn execute_pla(cpu: &mut Cpu) {
  cpu.a = cpu.pop_stack();
  cpu.update_acc_flags();
}

fn execute_plp(cpu: &mut Cpu) {
  let p = cpu.pop_stack();
  cpu.set_psr(p);
}

fn execute_rol(cpu: &mut Cpu, value: u8) -> u8 {
  let c = cpu.c as u8;
  cpu.c = value & 0x80 > 0;
  let result = (value << 1) | c;
  cpu.update_result_flags(result);
  result
}

fn execute_ror(cpu: &mut Cpu, value: u8) -> u8 {
  let mut result = value.rotate_right(1);
  if cpu.c {
    result |= 0x80;
  } else {
    result &= 0x7f;
  }
  cpu.c = value & 0x01 > 0;
  cpu.update_result_flags(result);
  result
}

fn execute_rti(cpu: &mut Cpu) {
  let p = cpu.pop_stack();
  cpu.set_psr(p);
  cpu.pc = cpu.pop_stack16();
}

fn execute_rts(cpu: &mut Cpu) {
  cpu.pc = cpu.pop_stack16().wrapping_add(1);
}

fn binary_sub(value1: u8, value2: u8, carry: bool) -> (u8, bool, bool) {
  let (n1, v1) = value1.overflowing_sub(value2);
  let (n2, v2) = n1.overflowing_sub(!carry as u8);
  let diff = (value1 as i8 as i16) - (value2 as i8 as i16) - (1 - (carry as i16));
  (n2, !(v1 | v2), (diff < -128) || (diff > 127))
}

fn execute_binary_sbc(cpu: &mut Cpu, value: u8) {
  let (result, carry, overflow) = binary_sub(cpu.a, value, cpu.c);
  cpu.a = result;
  cpu.c = carry;
  cpu.v = overflow;
  cpu.update_acc_flags();
}

fn execute_decimal_sbc(cpu: &mut Cpu, value: u8) {
  let mut al = ((cpu.a & 0x0f) as i8) - ((value & 0x0f) as i8) + (cpu.c as i8) - 1;
  if al < 0 {
    al = ((al - 0x06) & 0x0f) - 0x10;
  }
  let mut a = ((cpu.a & 0xf0) as i8 as i16) - ((value & 0xf0) as i8 as i16) + (al as i16);
  if a < 0 {
    a -= 0x60;
  }
  cpu.a = (a & 0x00ff) as u8;

  let (bin, carry, overflow) = binary_sub(cpu.a, value, cpu.c);
  cpu.c = carry;
  cpu.v = overflow;
  cpu.update_result_flags(bin);
}

fn execute_sbc(cpu: &mut Cpu, value: u8) {
  if cpu.d {
    execute_decimal_sbc(cpu, value);
  } else {
    execute_binary_sbc(cpu, value);
  }
}

fn execute_sec(cpu: &mut Cpu) {
  cpu.c = true;
}

fn execute_sed(cpu: &mut Cpu) {
  cpu.d = true;
}

fn execute_sei(cpu: &mut Cpu) {
  cpu.i = true;
}

fn execute_tax(cpu: &mut Cpu) {
  cpu.x = cpu.a;
  cpu.update_result_flags(cpu.x);
}

fn execute_tay(cpu: &mut Cpu) {
  cpu.y = cpu.a;
  cpu.update_result_flags(cpu.y);
}

fn execute_tsx(cpu: &mut Cpu) {
  cpu.x = cpu.sp;
  cpu.update_result_flags(cpu.x);
}

fn execute_txa(cpu: &mut Cpu) {
  cpu.a = cpu.x;
  cpu.update_acc_flags();
}

fn execute_txs(cpu: &mut Cpu) {
  cpu.sp = cpu.x;
}

fn execute_tya(cpu: &mut Cpu) {
  cpu.a = cpu.y;
  cpu.update_acc_flags();
}

// Undocumented instructions

fn execute_anc(cpu: &mut Cpu, value: u8) {
  let result = cpu.a & value;
  cpu.a = result;
  cpu.update_acc_flags();
  cpu.c = result & 0x80 > 0;
}

fn execute_dcp(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_dec(cpu, value);
  execute_cmp(cpu, result);
  result
}

fn execute_isc(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_inc(cpu, value);
  execute_sbc(cpu, result);
  result
}

fn execute_lax(cpu: &mut Cpu, value: u8) {
  cpu.a = value;
  cpu.x = value;
  cpu.update_acc_flags();
}

fn execute_rla(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_rol(cpu, value);
  execute_and(cpu, result);
  result
}

fn execute_rra(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_ror(cpu, value);
  execute_adc(cpu, result);
  result
}

fn execute_sax(cpu: &mut Cpu) -> u8 {
  cpu.a & cpu.x
}

fn execute_slo(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_asl(cpu, value);
  execute_ora(cpu, result);
  result
}

fn execute_sre(cpu: &mut Cpu, value: u8) -> u8 {
  let result = execute_lsr(cpu, value);
  execute_eor(cpu, result);
  result
}

fn execute_unimplemented(op: Operation) {
  panic!("Unimplemented operation: {:?}", op);
}

fn generates_read(op: Operation) -> bool {
  match op {
    STA => false,
    STX => false,
    STY => false,
    _ => true,
  }
}

fn crossed_page_boundary(ptr1: u16, ptr2: u16) -> bool {
  (ptr1 / 256) != (ptr2 / 256)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::cpu::Cpu;
  use crate::memory::Ram;

  const ADDRESS_TEST: u16 = 0xc000;

  fn create_test_cpu(program: &Vec<u8>) -> Cpu {
    let mut memory = Ram::new(65536);
    for (byte, index) in program.iter().zip(0..65536) {
      memory.write(ADDRESS_TEST + index as u16, *byte);
    }
    let mut cpu = Cpu::new(Box::new(memory));
    cpu.pc = ADDRESS_TEST;
    cpu
  }

  #[rustfmt::skip]
  fn with_acc_op_imm(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program = vec![
      0xa9, 0x00, // LDA #$00
      op,   0x00, // op  #$00
    ];
    (program, 2, 1, 3)
  }

  #[rustfmt::skip]
  fn with_acc_op_zpg(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00, // LDA #$00
      0x85, 0x2f, // STA $2F
      0xa9, 0x00, // LDA #$00
      op,   0x2f, // op  $2F
    ];
    (program, 4, 5, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_zpx(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00, // LDA #$00
      0x85, 0x2e, // STA $2E
      0xa9, 0x00, // LDA #$00
      0xa2, 0xff, // LDX #$FF
      op,   0x2f, // op  $2F,X
    ];
    (program, 5, 5, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_abs(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00,       // LDA #$00
      0x8d, 0xc1, 0x06, // STA $06C1
      0xa9, 0x00,       // LDA #$00
      op,   0xc1, 0x06, // op  $06C1
    ];
    (program, 4, 6, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_abx(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00,       // LDA #$00
      0x8d, 0xf0, 0x06, // STA $06F0
      0xa9, 0x00,       // LDA #$00
      0xa2, 0x2f,       // LDX #$2F
      op,   0xc1, 0x06, // op  $06C1,X
    ];
    (program, 5, 6, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_aby(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00,       // LDA #$00
      0x8d, 0xf0, 0x06, // STA $06F0
      0xa9, 0x00,       // LDA #$00
      0xa0, 0x2f,       // LDY #$2F
      op,   0xc1, 0x06, // op  $06C1,Y
    ];
    (program, 5, 6, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_inx(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00,       // LDA #$00
      0x8d, 0xc1, 0x06, // STA $06C1
      0xa9, 0xc1,       // LDA #$C1
      0x85, 0x2e,       // STA $2E
      0xa9, 0x06,       // LDA #$06
      0x85, 0x2f,       // STA $2F
      0xa9, 0x00,       // LDA #$00
      0xa2, 0xff,       // LDX #$FF
      op,   0x2f,       // op  ($2F,X)
    ];
    (program, 9, 14, 1)
  }

  #[rustfmt::skip]
  fn with_acc_op_iny(op: u8) -> (Vec<u8>, usize, u16, u16) {
    let program: Vec<u8> = vec![
      0xa9, 0x00,       // LDA #$00
      0x8d, 0xcc, 0x06, // STA $06CC
      0xa9, 0xc1,       // LDA #$C1
      0x85, 0x2f,       // STA $2F
      0xa9, 0x06,       // LDA #$06
      0x85, 0x30,       // STA $30
      0xa9, 0x00,       // LDA #$00
      0xa0, 0x0b,       // LDY #$0B
      op,   0x2f,       // op  ($2F),Y
    ];
    (program, 9, 14, 1)
  }

  fn run_adc_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        for c in (0 as u8)..=(1 as u8) {
          cpu.memory.write(ADDRESS_TEST + offset1, n1);
          cpu.memory.write(ADDRESS_TEST + offset2, n2);
          cpu.pc = ADDRESS_TEST;
          cpu.set_psr(c);
          cpu.run_instructions(len);

          let (temp, c1) = n1.overflowing_add(n2);
          let (result, c2) = temp.overflowing_add(c);
          let result2c = (n1 as i8 as i16) + (n2 as i8 as i16) + (c as i16);
          let carry = c1 | c2;
          let overflow = result2c < -128 || result2c > 127;
          let negative = result & 0x80 > 0;
          let zero = result == 0;

          assert_eq!(result, cpu.a);
          assert_eq!(negative, cpu.n);
          assert_eq!(overflow, cpu.v);
          assert_eq!(zero, cpu.z);
          assert_eq!(carry, cpu.c);
        }
      }
    }
  }

  #[test]
  fn adc_imm() {
    run_adc_tests(with_acc_op_imm(0x69));
  }

  #[test]
  fn adc_zpg() {
    run_adc_tests(with_acc_op_zpg(0x65));
  }

  #[test]
  fn adc_zpx() {
    run_adc_tests(with_acc_op_zpx(0x75));
  }

  #[test]
  fn adc_abs() {
    run_adc_tests(with_acc_op_abs(0x6d));
  }

  #[test]
  fn adc_abx() {
    run_adc_tests(with_acc_op_abx(0x7d));
  }

  #[test]
  fn adc_aby() {
    run_adc_tests(with_acc_op_aby(0x79));
  }

  #[test]
  fn adc_inx() {
    run_adc_tests(with_acc_op_inx(0x61));
  }

  #[test]
  fn adc_iny() {
    run_adc_tests(with_acc_op_iny(0x71));
  }

  fn run_and_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        cpu.memory.write(ADDRESS_TEST + offset1, n1);
        cpu.memory.write(ADDRESS_TEST + offset2, n2);
        cpu.pc = ADDRESS_TEST;
        cpu.set_psr(0x00);
        cpu.run_instructions(len);

        let result = n1 & n2;
        let negative = result & 0x80 > 0;
        let zero = result == 0;

        assert_eq!(result, cpu.a);
        assert_eq!(negative, cpu.n);
        assert_eq!(zero, cpu.z);
      }
    }
  }

  #[test]
  fn and_imm() {
    run_and_tests(with_acc_op_imm(0x29));
  }

  #[test]
  fn and_zpg() {
    run_and_tests(with_acc_op_zpg(0x25));
  }

  #[test]
  fn and_zpx() {
    run_and_tests(with_acc_op_zpx(0x35));
  }

  #[test]
  fn and_abs() {
    run_and_tests(with_acc_op_abs(0x2d));
  }

  #[test]
  fn and_abx() {
    run_and_tests(with_acc_op_abx(0x3d));
  }

  #[test]
  fn and_aby() {
    run_and_tests(with_acc_op_aby(0x39));
  }

  #[test]
  fn and_inx() {
    run_and_tests(with_acc_op_inx(0x21));
  }

  #[test]
  fn and_iny() {
    run_and_tests(with_acc_op_iny(0x31));
  }

  fn run_cmp_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        cpu.memory.write(ADDRESS_TEST + offset1, n1);
        cpu.memory.write(ADDRESS_TEST + offset2, n2);
        cpu.pc = ADDRESS_TEST;
        cpu.set_psr(0x00);
        cpu.run_instructions(len);

        let negative = n1.wrapping_sub(n2) & 0x80 > 0;
        let zero = n1 == n2;
        let carry = n1 >= n2;

        assert_eq!(n1, cpu.a);
        assert_eq!(negative, cpu.n);
        assert_eq!(zero, cpu.z);
        assert_eq!(carry, cpu.c);
      }
    }
  }

  #[test]
  fn cmp_imm() {
    run_cmp_tests(with_acc_op_imm(0xc9));
  }

  #[test]
  fn cmp_zpg() {
    run_cmp_tests(with_acc_op_zpg(0xc5));
  }

  #[test]
  fn cmp_zpx() {
    run_cmp_tests(with_acc_op_zpx(0xd5));
  }

  #[test]
  fn cmp_abs() {
    run_cmp_tests(with_acc_op_abs(0xcd));
  }

  #[test]
  fn cmp_abx() {
    run_cmp_tests(with_acc_op_abx(0xdd));
  }

  #[test]
  fn cmp_aby() {
    run_cmp_tests(with_acc_op_aby(0xd9));
  }

  #[test]
  fn cmp_inx() {
    run_cmp_tests(with_acc_op_inx(0xc1));
  }

  #[test]
  fn cmp_iny() {
    run_cmp_tests(with_acc_op_iny(0xd1));
  }

  fn run_eor_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        cpu.memory.write(ADDRESS_TEST + offset1, n1);
        cpu.memory.write(ADDRESS_TEST + offset2, n2);
        cpu.pc = ADDRESS_TEST;
        cpu.set_psr(0x00);
        cpu.run_instructions(len);

        let result = n1 ^ n2;
        let negative = result & 0x80 > 0;
        let zero = result == 0;

        assert_eq!(result, cpu.a);
        assert_eq!(negative, cpu.n);
        assert_eq!(zero, cpu.z);
      }
    }
  }

  #[test]
  fn eor_imm() {
    run_eor_tests(with_acc_op_imm(0x49));
  }

  #[test]
  fn eor_zpg() {
    run_eor_tests(with_acc_op_zpg(0x45));
  }

  #[test]
  fn eor_zpx() {
    run_eor_tests(with_acc_op_zpx(0x55));
  }

  #[test]
  fn eor_abs() {
    run_eor_tests(with_acc_op_abs(0x4d));
  }

  #[test]
  fn eor_abx() {
    run_eor_tests(with_acc_op_abx(0x5d));
  }

  #[test]
  fn eor_aby() {
    run_eor_tests(with_acc_op_aby(0x59));
  }

  #[test]
  fn eor_inx() {
    run_eor_tests(with_acc_op_inx(0x41));
  }

  #[test]
  fn eor_iny() {
    run_eor_tests(with_acc_op_iny(0x51));
  }

  fn run_ora_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        cpu.memory.write(ADDRESS_TEST + offset1, n1);
        cpu.memory.write(ADDRESS_TEST + offset2, n2);
        cpu.pc = ADDRESS_TEST;
        cpu.set_psr(0x00);
        cpu.run_instructions(len);

        let result = n1 | n2;
        let negative = result & 0x80 > 0;
        let zero = result == 0;

        assert_eq!(result, cpu.a);
        assert_eq!(negative, cpu.n);
        assert_eq!(zero, cpu.z);
      }
    }
  }

  #[test]
  fn ora_imm() {
    run_ora_tests(with_acc_op_imm(0x09));
  }

  #[test]
  fn ora_zpg() {
    run_ora_tests(with_acc_op_zpg(0x05));
  }

  #[test]
  fn ora_zpx() {
    run_ora_tests(with_acc_op_zpx(0x15));
  }

  #[test]
  fn ora_abs() {
    run_ora_tests(with_acc_op_abs(0x0d));
  }

  #[test]
  fn ora_abx() {
    run_ora_tests(with_acc_op_abx(0x1d));
  }

  #[test]
  fn ora_aby() {
    run_ora_tests(with_acc_op_aby(0x19));
  }

  #[test]
  fn ora_inx() {
    run_ora_tests(with_acc_op_inx(0x01));
  }

  #[test]
  fn ora_iny() {
    run_ora_tests(with_acc_op_iny(0x11));
  }

  fn run_sbc_tests((program, len, offset1, offset2): (Vec<u8>, usize, u16, u16)) {
    let mut cpu = create_test_cpu(&program);

    for n1 in (0 as u8)..=(255 as u8) {
      for n2 in (0 as u8)..=(255 as u8) {
        for c in (0 as u8)..=(1 as u8) {
          cpu.memory.write(ADDRESS_TEST + offset1, n1);
          cpu.memory.write(ADDRESS_TEST + offset2, n2);
          cpu.pc = ADDRESS_TEST;
          cpu.set_psr(c);
          cpu.run_instructions(len);

          let (temp, c1) = n1.overflowing_sub(n2);
          let (result, c2) = temp.overflowing_sub(1 - c);
          let result2c = (n1 as i8 as i16) - (n2 as i8 as i16) - (1 - (c as i16));
          let carry = !(c1 | c2);
          let overflow = result2c < -128 || result2c > 127;
          let negative = result & 0x80 > 0;
          let zero = result == 0;

          assert_eq!(result, cpu.a);
          assert_eq!(negative, cpu.n);
          assert_eq!(overflow, cpu.v);
          assert_eq!(zero, cpu.z);
          assert_eq!(carry, cpu.c);
        }
      }
    }
  }

  #[test]
  fn sbc_imm() {
    run_sbc_tests(with_acc_op_imm(0xe9));
  }

  #[test]
  fn sbc_zpg() {
    run_sbc_tests(with_acc_op_zpg(0xe5));
  }

  #[test]
  fn sbc_zpx() {
    run_sbc_tests(with_acc_op_zpx(0xf5));
  }

  #[test]
  fn sbc_abs() {
    run_sbc_tests(with_acc_op_abs(0xed));
  }

  #[test]
  fn sbc_abx() {
    run_sbc_tests(with_acc_op_abx(0xfd));
  }

  #[test]
  fn sbc_aby() {
    run_sbc_tests(with_acc_op_aby(0xf9));
  }

  #[test]
  fn sbc_inx() {
    run_sbc_tests(with_acc_op_inx(0xe1));
  }

  #[test]
  fn sbc_iny() {
    run_sbc_tests(with_acc_op_iny(0xf1));
  }
}
