use super::{decoder, encoder, Frame};

use bytes::BytesMut;

use tokio::io::{AsyncWrite, AsyncRead};
use tokio_util::codec::Framed;


#[derive(Debug)]
pub enum DecoderError {
    Io(std::io::Error),
    Decoder(decoder::Error),
}

impl From<std::io::Error> for DecoderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<decoder::Error> for DecoderError {
    fn from(value: decoder::Error) -> Self {
        Self::Decoder(value)
    }
}


#[derive(Debug, Default)]
pub struct Codec {
    dec: decoder::Decoder,
}

impl Codec {
    pub fn new() -> Self {
        Self { dec: decoder::Decoder::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { dec: decoder::Decoder::with_capacity(cap) }
    }

    pub fn wrap<T>(self, io: T) -> Framed<T, Codec>
    where
        T: AsyncRead + AsyncWrite,
    {
        Framed::with_capacity(io, self, 4096 as _)
    }
}

impl tokio_util::codec::Encoder<&Frame> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, frame: &Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encoder::encode(dst, frame);
        Ok(())
    }
}

impl tokio_util::codec::Decoder for Codec {
    type Item = Frame;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.dec.process(src) {
            Ok(x) => Ok(x),
            Err(e) => {
                log::warn!("error decoding data: {e:?}");
                Ok(None)
            },
        }
    }
}
