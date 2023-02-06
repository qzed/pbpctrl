use super::Message;

use bytes::{Buf, BytesMut, BufMut};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Framed, Encoder};


const MAX_FRAME_SIZE: u16 = 4096;


pub struct Codec {}

impl Codec {
    pub fn new() -> Self {
        Self {}
    }

    pub fn wrap<T>(self, io: T) -> Framed<T, Codec>
    where
        T: AsyncRead + AsyncWrite,
    {
        Framed::with_capacity(io, self, MAX_FRAME_SIZE as _)
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let group = src[0];
        let code = src[1];

        let mut length = [0; 2];
        length.copy_from_slice(&src[2..4]);
        let length = u16::from_be_bytes(length);

        if length > MAX_FRAME_SIZE {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame of length {length} is too large (group: {group}, code: {code})."),
            ))?;
        }

        let size = 4 + length as usize;

        if src.len() < size as _ {
            src.reserve(size - src.len());
            return Ok(None);
        }

        let data = src[4..size].into();
        src.advance(size);

        Ok(Some(Message {
            group,
            code,
            data,
        }))
    }
}

impl Encoder<&Message> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, msg: &Message, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let size = msg.data.len() + 4;

        if size > MAX_FRAME_SIZE as usize {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Frame of length {size} is too large."),
            ))?;
        }

        buf.reserve(size);
        buf.put_u8(msg.group);
        buf.put_u8(msg.code);
        buf.put_slice(&(msg.data.len() as u16).to_be_bytes());
        buf.put_slice(&msg.data);

        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::msg::{EventGroup, DeviceEventCode, Message};

    use bytes::BytesMut;

    use smallvec::smallvec;


    #[test]
    fn test_encode() {
        let mut buf = BytesMut::new();
        let mut codec = Codec::new();

        let msg = Message {
            group: EventGroup::Device.into(),
            code: DeviceEventCode::ModelId.into(),
            data: smallvec![0x00, 0x01, 0x02, 0x04, 0x05],
        };

        // try to encode the message
        codec.encode(&msg, &mut buf)
            .expect("error encode message");

        let raw = [0x03, 0x01, 0x00, 0x05, 0x00, 0x01, 0x02, 0x04, 0x05];
        assert_eq!(&buf[..], &raw[..]);
    }

    #[test]
    fn test_decode() {
        let mut codec = Codec::new();

        let raw = [0x03, 0x01, 0x00, 0x03, 0x00, 0x01, 0x02];
        let mut buf = BytesMut::from(&raw[..]);

        let msg = Message {
            group: EventGroup::Device.into(),
            code: DeviceEventCode::ModelId.into(),
            data: smallvec![0x00, 0x01, 0x02],
        };

        // try to encode the message
        let decoded = codec.decode(&mut buf)
            .expect("error decoding message")
            .expect("message incomplete");

        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_decode_incomplete() {
        let mut codec = Codec::new();

        let raw = [0x03, 0x01, 0x00, 0x03, 0x00];
        let mut buf = BytesMut::from(&raw[..]);

        // try to encode the message
        let decoded = codec.decode(&mut buf)
            .expect("error decoding message");

        assert_eq!(decoded, None);
    }

    #[test]
    fn test_encode_decode() {
        let mut buf = BytesMut::new();
        let mut codec = Codec::new();

        let msg = Message {
            group: 0,
            code: 0,
            data: smallvec![0x00, 0x01, 0x02],
        };

        // try to encode the message
        codec.encode(&msg, &mut buf)
            .expect("error encode message");

        // try to decode the message we just encoded
        let decoded = codec.decode(&mut buf)
            .expect("error decoding message")
            .expect("message incomplete");
        
        assert_eq!(decoded, msg);
    }
}
