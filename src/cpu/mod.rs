mod alu;
mod instruction;

use crate::common::Clocked;
use crate::cpu::alu::decode_instruction;
use crate::cpu::alu::execute_instruction;
use crate::memory::Addressable;
use crate::save::Saveable;
use std::io::Read;
use std::io::Result;
use std::io::Write;

const ADDRESS_NMI: u16 = 0xfffa;
const ADDRESS_RESET: u16 = 0xfffc;
const ADDRESS_BRK: u16 = 0xfffe;

const STACK_PAGE: u16 = 0x0100;

pub struct Cpu {
  a: u8,
  x: u8,
  y: u8,
  pc: u16,
  sp: u8,
  c: bool,
  z: bool,
  i: bool,
  d: bool,
  v: bool,
  n: bool,
  pub memory: Box<dyn Addressable>,
  pub counter: usize,
  pub cycles: usize,
  cycles_left: u16,
}

impl Cpu {
  pub fn new(memory: Box<dyn Addressable>) -> Cpu {
    Cpu {
      a: 0,
      x: 0,
      y: 0,
      pc: ADDRESS_RESET,
      sp: 0xfd,
      c: false,
      z: false,
      i: false,
      d: false,
      v: false,
      n: false,
      memory,
      counter: 0,
      cycles: 0,
      cycles_left: 6,
    }
  }
}

impl Saveable for Cpu {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    self.a.save(handle)?;
    self.x.save(handle)?;
    self.y.save(handle)?;
    self.pc.save(handle)?;
    self.sp.save(handle)?;
    self.c.save(handle)?;
    self.z.save(handle)?;
    self.i.save(handle)?;
    self.d.save(handle)?;
    self.v.save(handle)?;
    self.n.save(handle)?;
    self.memory.save(handle)?;
    self.counter.save(handle)?;
    self.cycles.save(handle)?;
    self.cycles_left.save(handle)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    self.a.load(handle)?;
    self.x.load(handle)?;
    self.y.load(handle)?;
    self.pc.load(handle)?;
    self.sp.load(handle)?;
    self.c.load(handle)?;
    self.z.load(handle)?;
    self.i.load(handle)?;
    self.d.load(handle)?;
    self.v.load(handle)?;
    self.n.load(handle)?;
    self.memory.load(handle)?;
    self.counter.load(handle)?;
    self.cycles.load(handle)?;
    self.cycles_left.load(handle)?;
    Ok(())
  }
}

impl Addressable for Cpu {
  fn read(&self, ptr: u16) -> u8 {
    self.memory.read(ptr)
  }

  fn write(&mut self, ptr: u16, value: u8) {
    self.memory.write(ptr, value);
  }
}

impl Clocked for Cpu {
  fn clock(&mut self) {
    self.counter += 1;
    if self.cycles_left > 0 {
      self.cycles_left -= 1;
      return;
    }

    let (i, byte_count, paged) = decode_instruction(self);
    self.pc = self.pc.wrapping_add(byte_count);
    execute_instruction(self, i);

    let extra_cycles = if i.page_cycle && paged { 1 } else { 0 };
    self.cycles_left += (i.cycles as u16) + extra_cycles - 1;
  }
}

impl Cpu {
  pub fn run_instructions(&mut self, n: usize) {
    for _ in 0..n {
      self.cycles_left = 0;
      self.clock();
    }
  }

  pub fn nmi(&mut self) {
    let pc = self.pc;
    let status = self.get_psr(false);

    self.push_stack16(pc);
    self.push_stack(status);
    self.pc = self.read16(ADDRESS_NMI);
  }

  pub fn irq(&mut self) {
    if self.i {
      return;
    }
    let pc = self.pc;
    let status = self.get_psr(false);

    self.push_stack16(pc);
    self.push_stack(status);
    self.pc = self.read16(ADDRESS_BRK);
  }

  pub fn pause(&mut self, cycles: u16) {
    self.cycles_left = cycles;
  }

  fn read_target(&self, target: Option<u16>) -> u8 {
    match target {
      None => self.a,
      Some(ptr) => self.read(ptr),
    }
  }

  fn write_target(&mut self, target: Option<u16>, value: u8) {
    match target {
      None => self.a = value,
      Some(ptr) => self.write(ptr, value),
    }
  }
}

// STACK

impl Cpu {
  fn push_stack(&mut self, value: u8) {
    self.write_offset(STACK_PAGE, self.sp as u16, value);
    self.sp = self.sp.wrapping_sub(1);
  }

  fn pop_stack(&mut self) -> u8 {
    self.sp = self.sp.wrapping_add(1);
    self.read_offset(STACK_PAGE, self.sp as u16)
  }

  fn push_stack16(&mut self, value: u16) {
    self.push_stack(((value & 0xff00) >> 8) as u8);
    self.push_stack((value & 0xff) as u8);
  }

  fn pop_stack16(&mut self) -> u16 {
    let lsb = self.pop_stack() as u16;
    let msb = self.pop_stack() as u16;
    (msb << 8) | lsb
  }

  fn read_zero16(&self, ptr: u8) -> u16 {
    let lsb = self.read(ptr as u16) as u16;
    let msb = self.read(ptr.wrapping_add(1) as u16) as u16;
    (msb << 8) | lsb
  }

  fn read_pagewrap16(&self, ptr: u16) -> u16 {
    let lsb = self.read(ptr) as u16;
    let msb = self.read(((ptr >> 8) << 8) | (((ptr % 256) as u8).wrapping_add(1) as u16)) as u16;
    (msb << 8) | lsb
  }
}

// FLAGS

impl Cpu {
  fn update_result_flags(&mut self, value: u8) {
    self.z = value == 0;
    self.n = value >= 128;
  }

  fn update_acc_flags(&mut self) {
    self.update_result_flags(self.a);
  }

  fn get_psr(&self, is_instruction: bool) -> u8 {
    (self.c as u8)
      | ((self.z as u8) << 1)
      | ((self.i as u8) << 2)
      | ((self.d as u8) << 3)
      | ((if is_instruction { 1 } else { 0 }) << 5)
      | ((self.v as u8) << 6)
      | ((self.n as u8) << 7)
  }

  fn set_psr(&mut self, value: u8) {
    self.c = value & 0x01 > 0;
    self.z = value & 0x02 > 0;
    self.i = value & 0x04 > 0;
    self.d = value & 0x08 > 0;
    self.v = value & 0x40 > 0;
    self.n = value & 0x80 > 0;
  }
}
