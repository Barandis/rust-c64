use crate::save::Saveable;
use std::io::Read;
use std::io::Result;
use std::io::Write;

pub trait Addressable: Saveable {
  fn read(&self, ptr: u16) -> u8;
  fn write(&mut self, ptr: u16, value: u8);

  fn read16(&self, ptr: u16) -> u16 {
    let lo = self.read(ptr);
    let hi = self.read(ptr.wrapping_add(1));
    (lo as u16) + ((hi as u16) << 8)
  }

  fn read_offset(&self, ptr: u16, offset: u16) -> u8 {
    self.read(ptr.wrapping_add(offset))
  }

  fn read_offset16(&self, ptr: u16, offset: u16) -> u16 {
    self.read16(ptr.wrapping_add(offset))
  }

  fn write_offset(&mut self, ptr: u16, offset: u16, value: u8) {
    self.write(ptr.wrapping_add(offset), value);
  }
}

pub struct Ram {
  bytes: Vec<u8>,
}

impl Ram {
  pub fn new(size: usize) -> Ram {
    Ram {
      bytes: vec![0; size],
    }
  }
}

impl Saveable for Ram {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    self.bytes.save(handle)
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    self.bytes.load(handle)
  }
}

impl Addressable for Ram {
  fn read(&self, ptr: u16) -> u8 {
    self.bytes[ptr as usize]
  }

  fn write(&mut self, ptr: u16, value: u8) {
    self.bytes[ptr as usize] = value;
  }
}

pub struct Rom {
  bytes: Vec<u8>,
}

impl Rom {
  pub fn new(bytes: Vec<u8>) -> Rom {
    Rom { bytes }
  }
}

impl Saveable for Rom {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    self.bytes.save(handle)
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    self.bytes.load(handle)
  }
}

impl Addressable for Rom {
  fn read(&self, ptr: u16) -> u8 {
    self.bytes[ptr as usize]
  }

  fn write(&mut self, ptr: u16, value: u8) {
    panic!("Attempt to write to read-only memory at {}: {}", ptr, value);
  }
}
