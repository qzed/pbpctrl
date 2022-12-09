//! Types for GFPS Message Stream via RFCOMM.

use uuid::{uuid, Uuid};

mod codec;
mod types;

pub use codec::Codec;
pub use types::*;

/// UUID under which the GFPS Message Stream is advertised.
/// 
/// Defined as `df21fe2c-2515-4fdb-8886-f12c4d67927c`.
pub const UUID: Uuid = uuid!("df21fe2c-2515-4fdb-8886-f12c4d67927c");
