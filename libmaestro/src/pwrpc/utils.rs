//! Miscellaneous utilities and helpers.

use bytes::{Buf, BufMut};

/// An encoded protobuf message.
///
/// This type represents an encoded protobuf message. Decoding and encoding are
/// essentially no-ops, reading and writing to/from the internal buffer. It is
/// a drop-in replacement for any valid (and invalid) protobuf type.
///
/// This type is intended for reverse-engineering and testing, e.g., in
/// combination with tools like `protoscope`.
#[derive(Clone, Default)]
pub struct EncodedMessage {
    pub data: Vec<u8>,
}

impl std::fmt::Debug for EncodedMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02x?}", self.data)
    }
}

impl prost::Message for EncodedMessage {
    fn encode_raw(&self, buf: &mut impl BufMut) {
        buf.put_slice(&self.data[..])
    }

    fn merge_field(
        &mut self,
        _tag: u32,
        _wire_type: prost::encoding::WireType,
        _buf: &mut impl Buf,
        _ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        unimplemented!("use merge() instead")
    }

    fn merge(&mut self, mut buf: impl Buf) -> Result<(), prost::DecodeError> {
        let a = self.data.len();
        let b = a + buf.remaining();

        self.data.resize(b, 0);
        buf.copy_to_slice(&mut self.data[a..b]);

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.data.len()
    }

    fn clear(&mut self) {
        self.data.clear()
    }
}
