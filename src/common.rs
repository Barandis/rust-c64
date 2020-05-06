pub trait Clocked {
  fn clock(&mut self);
}

pub fn get_bit(num: u8, idx: u8) -> bool {
  (num >> idx) & 1 > 0
}