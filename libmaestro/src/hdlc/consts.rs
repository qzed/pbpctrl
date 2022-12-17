//! Flag bytes and bit masks used in the HDLC encoding.

pub mod flags {
    pub const FRAME: u8 = 0x7E;
    pub const ESCAPE: u8 = 0x7D;
}

pub mod escape {
    pub const MASK: u8 = 0x20;
}
