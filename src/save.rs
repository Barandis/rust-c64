use std::io::Read;
use std::io::Result;
use std::io::Write;

pub trait Saveable {
  fn save(&self, handle: &mut dyn Write) -> Result<()>;
  fn load(&mut self, handle: &mut dyn Read) -> Result<()>;
}

impl Saveable for bool {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    let bytes = [*self as u8];
    handle.write_all(&bytes)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut bytes = [0];
    handle.read_exact(&mut bytes)?;
    *self = bytes[0] > 0;
    Ok(())
  }
}

impl Saveable for u8 {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    let bytes = [*self as u8];
    handle.write_all(&bytes)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut bytes = [0u8];
    handle.read_exact(&mut bytes)?;
    *self = bytes[0];
    Ok(())
  }
}

impl Saveable for u16 {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    let bytes = [(*self & 0xff) as u8, ((*self >> 8) & 0xff) as u8];
    handle.write_all(&bytes)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut bytes = [0u8; 2];
    handle.read_exact(&mut bytes)?;
    *self = bytes[0] as u16;
    *self |= (bytes[1] as u16) << 8;
    Ok(())
  }
}

impl Saveable for u32 {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    let bytes = [
      (*self & 0xff) as u8,
      ((*self >> 8) & 0xff) as u8,
      ((*self >> 16) & 0xff) as u8,
      ((*self >> 24) & 0xff) as u8,
    ];
    handle.write_all(&bytes)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut bytes = [0u8; 4];
    handle.read_exact(&mut bytes)?;
    *self = bytes[0] as u32;
    *self |= (bytes[1] as u32) << 8;
    *self |= (bytes[2] as u32) << 16;
    *self |= (bytes[3] as u32) << 24;
    Ok(())
  }
}

impl Saveable for u64 {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    let bytes = [
      (*self & 0xff) as u8,
      ((*self >> 8) & 0xff) as u8,
      ((*self >> 16) & 0xff) as u8,
      ((*self >> 24) & 0xff) as u8,
      ((*self >> 32) & 0xff) as u8,
      ((*self >> 40) & 0xff) as u8,
      ((*self >> 48) & 0xff) as u8,
      ((*self >> 56) & 0xff) as u8,
    ];
    handle.write_all(&bytes)?;
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut bytes = [0u8; 8];
    handle.read_exact(&mut bytes)?;
    *self = bytes[0] as u64;
    *self |= (bytes[1] as u64) << 8;
    *self |= (bytes[2] as u64) << 16;
    *self |= (bytes[3] as u64) << 24;
    *self |= (bytes[4] as u64) << 32;
    *self |= (bytes[5] as u64) << 40;
    *self |= (bytes[6] as u64) << 48;
    *self |= (bytes[7] as u64) << 56;
    Ok(())
  }
}

impl Saveable for usize {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    (*self as u64).save(handle)
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut a = *self as u64;
    a.load(handle)?;
    *self = a as usize;
    Ok(())
  }
}

impl<T: Saveable> Saveable for [T] {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    self.len().save(handle)?;
    for i in self.iter() {
      i.save(handle)?;
    }
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut len = 0usize;
    len.load(handle)?;
    for i in 0..len {
      self[i].load(handle)?;
    }
    Ok(())
  }
}

impl<T: Saveable + Default> Saveable for Vec<T> {
  fn save(&self, handle: &mut dyn Write) -> Result<()> {
    self.len().save(handle)?;
    for i in self.iter() {
      i.save(handle)?;
    }
    Ok(())
  }

  fn load(&mut self, handle: &mut dyn Read) -> Result<()> {
    let mut len = 0usize;
    len.load(handle)?;
    self.truncate(0);
    self.reserve(len);
    for _ in 0..len {
      let a: T = read_value(handle)?;
      self.push(a);
    }
    Ok(())
  }
}

pub fn read_value<T>(handle: &mut dyn Read) -> Result<T>
where
  T: Default + Saveable,
{
  let mut a: T = Default::default();
  a.load(handle)?;
  Ok(a)
}
