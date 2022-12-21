use num_enum::{IntoPrimitive, FromPrimitive};

#[non_exhaustive]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum RpcStatus {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,

    #[num_enum(catch_all)]
    Unsupported(u32),
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcType {
    Unary,
    ServerStream,
    ClientStream,
    BidirectionalStream,
}

impl RpcType {
    pub fn has_server_stream(&self) -> bool {
        match *self {
            RpcType::ServerStream | RpcType::BidirectionalStream => true,
            RpcType::Unary | RpcType::ClientStream => false,
        }
    }

    pub fn has_client_stream(&self) -> bool {
        match *self {
            RpcType::ClientStream | RpcType::BidirectionalStream => true,
            RpcType::Unary | RpcType::ServerStream => false,
        }
    }
}


mod generated {
    include!(concat!(env!("OUT_DIR"), "/pw.rpc.packet.rs"));
}

pub use generated::PacketType;
pub use generated::RpcPacket;
