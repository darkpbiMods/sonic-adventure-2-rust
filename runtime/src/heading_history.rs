//! Per-player256-slot position/angle ring used by8004f56c and80038008.
//! Caller owns player selection, initialization and end-of-update scheduling.
pub struct HeadingHistory {
 pub index:u8,
 pub positions:[[u32;3];256],
 pub angles:[[u32;3];256],
}
impl HeadingHistory {
 /// Fresh calloc-backed bank. Index initialization is a separate caller concern;
 /// existing source buffers survive allocation setup and must not be reseeded.
 pub fn zeroed(index:u8)->Self{Self{index,positions:[[0;3];256],angles:[[0;3];256]}}
 pub fn record(&mut self,position:[u32;3],angles:[u32;3]){
  self.index=self.index.wrapping_add(1);
  self.positions[self.index as usize]=position;self.angles[self.index as usize]=angles;
 }
 pub fn delayed_angles(&self,delay:u8)->[u32;3]{self.angles[self.index.wrapping_sub(delay)as usize]}
}
