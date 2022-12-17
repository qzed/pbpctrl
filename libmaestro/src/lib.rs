//! Library for the Maestro protocol used to change settings (ANC, equalizer,
//! etc.) on the Google Pixel Buds Pro. Might support other Pixel Buds, might
//! not.

use uuid::{uuid, Uuid};

/// UUID under which the Maestro protocol is advertised.
///
/// Defined as `25e97ff7-24ce-4c4c-8951-f764a708f7b5`.
pub const UUID: Uuid = uuid!("25e97ff7-24ce-4c4c-8951-f764a708f7b5");

pub mod hdlc;
