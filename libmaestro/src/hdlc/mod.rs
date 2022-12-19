//! High-level Data Link Control (HDLC) support library.

pub mod consts;
pub mod crc;
pub mod decoder;
pub mod encoder;
pub mod varint;

use bytes::BytesMut;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub address: u32,
    pub control: u8,
    pub data: Box<[u8]>,
}

impl Frame {
    pub fn decode(buf: &mut BytesMut) -> Result<Option<Self>, decoder::Error> {
        decoder::Decoder::new().process(buf)
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        encoder::encode(buf, self)
    }

    pub fn encode_bytes(&self) -> BytesMut {
        encoder::encode_bytes(self)
    }
}
