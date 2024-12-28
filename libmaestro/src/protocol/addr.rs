use num_enum::{FromPrimitive, IntoPrimitive};


#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
pub enum Peer {
    Unknown = 0,
    Host = 1,
    Case = 2,
    LeftBtCore = 3,
    RightBtCore = 4,
    LeftSensorHub = 5,
    RightSensorHub = 6,
    LeftSpiBridge = 7,
    RightSpiBridge = 8,
    DebugApp = 9,
    MaestroA = 10,
    LeftTahiti = 11,
    RightTahiti = 12,
    MaestroB = 13,

    #[num_enum(catch_all)]
    Unrecognized(u8) = 0xff,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    value: u32,
}

impl Address {
    pub fn from_value(value: u32) -> Self {
        Address { value }
    }

    pub fn from_peers(source: Peer, target: Peer) -> Self {
        let source: u8 = source.into();
        let target: u8 = target.into();

        Self::from_value(((source as u32 & 0xf) << 6) | ((target as u32 & 0xf) << 10))
    }

    pub fn value(&self) -> u32 {
        self.value
    }

    pub fn source(&self) -> Peer {
        Peer::from_primitive(((self.value >> 6) & 0x0f) as u8)
    }

    pub fn target(&self) -> Peer {
        Peer::from_primitive(((self.value >> 10) & 0x0f) as u8)
    }

    pub fn swap(&self) -> Self {
        Self::from_peers(self.target(), self.source())
    }

    pub fn channel_id(&self) -> Option<u32> {
        let source = self.source();
        let target = self.target();

        if source == Peer::MaestroA || source == Peer::MaestroB {
            channel_id(source, target)
        } else {
            channel_id(target, source)
        }
    }
}

impl From<u32> for Address {
    fn from(value: u32) -> Self {
        Self::from_value(value)
    }
}

impl From<(Peer, Peer)> for Address {
    fn from(peers: (Peer, Peer)) -> Self {
        Self::from_peers(peers.0, peers.1)
    }
}


pub fn channel_id(local: Peer, remote: Peer) -> Option<u32> {
    match (local, remote) {
        (Peer::MaestroA, Peer::Case)           => Some(18),
        (Peer::MaestroA, Peer::LeftBtCore)     => Some(19),
        (Peer::MaestroA, Peer::LeftSensorHub)  => Some(20),
        (Peer::MaestroA, Peer::RightBtCore)    => Some(21),
        (Peer::MaestroA, Peer::RightSensorHub) => Some(22),
        (Peer::MaestroB, Peer::Case)           => Some(23),
        (Peer::MaestroB, Peer::LeftBtCore)     => Some(24),
        (Peer::MaestroB, Peer::LeftSensorHub)  => Some(25),
        (Peer::MaestroB, Peer::RightBtCore)    => Some(26),
        (Peer::MaestroB, Peer::RightSensorHub) => Some(27),
        (_, _) => None,
    }
}

pub fn address_for_channel(channel: u32) -> Option<Address> {
    match channel {
        18 => Some(Address::from_peers(Peer::MaestroA, Peer::Case)),
        19 => Some(Address::from_peers(Peer::MaestroA, Peer::LeftBtCore)),
        20 => Some(Address::from_peers(Peer::MaestroA, Peer::LeftSensorHub)),
        21 => Some(Address::from_peers(Peer::MaestroA, Peer::RightBtCore)),
        22 => Some(Address::from_peers(Peer::MaestroA, Peer::RightSensorHub)),
        23 => Some(Address::from_peers(Peer::MaestroB, Peer::Case)),
        24 => Some(Address::from_peers(Peer::MaestroB, Peer::LeftBtCore)),
        25 => Some(Address::from_peers(Peer::MaestroB, Peer::LeftSensorHub)),
        26 => Some(Address::from_peers(Peer::MaestroB, Peer::RightBtCore)),
        27 => Some(Address::from_peers(Peer::MaestroB, Peer::RightSensorHub)),
        _ => None,
    }
}
