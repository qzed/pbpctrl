//! High-level Data Link Control (HDLC) support library.


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub address: u32,
    pub control: u8,
    pub data: Box<[u8]>,
}
