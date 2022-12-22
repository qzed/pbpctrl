#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
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
}

impl Status {
    pub fn description(&self) -> &'static str {
        match self {
            Status::Ok => "The operation completed successfully",
            Status::Cancelled => "The operation was cancelled",
            Status::Unknown => "Unknown error",
            Status::InvalidArgument => "Client specified an invalid argument",
            Status::DeadlineExceeded => "Deadline expired before operation could complete",
            Status::NotFound => "Some requested entity was not found",
            Status::AlreadyExists => "Some entity that we attempted to create already exists",
            Status::PermissionDenied => "The caller does not have permission to execute the specified operation",
            Status::ResourceExhausted => "Some resource has been exhausted",
            Status::FailedPrecondition => "The system is not in a state required for the operation's execution",
            Status::Aborted => "The operation was aborted",
            Status::OutOfRange => "Operation was attempted past the valid range",
            Status::Unimplemented => "Operation is not implemented or not supported",
            Status::Internal => "Internal error",
            Status::Unavailable => "The service is currently unavailable",
            Status::DataLoss => "Unrecoverable data loss or corruption",
            Status::Unauthenticated => "The request does not have valid authentication credentials",
        }
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<u32> for Status {
    fn from(value: u32) -> Self {
        match value {
            0 => Status::Ok,
            1 => Status::Cancelled,
            2 => Status::Unknown,
            3 => Status::InvalidArgument,
            4 => Status::DeadlineExceeded,
            5 => Status::NotFound,
            6 => Status::AlreadyExists,
            7 => Status::PermissionDenied,
            8 => Status::ResourceExhausted,
            9 => Status::FailedPrecondition,
            10 => Status::Aborted,
            11 => Status::OutOfRange,
            12 => Status::Unimplemented,
            13 => Status::Internal,
            14 => Status::Unavailable,
            15 => Status::DataLoss,
            16 => Status::Unauthenticated,
            _ => Status::Unknown,
        }
    }
}

impl From<Status> for u32 {
    fn from(value: Status) -> Self {
        value as _
    }
}


#[derive(Debug)]
pub struct Error {
    code: Status,
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl Error {
    pub fn new(code: Status, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(Status::Cancelled, message)
    }

    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(Status::Unknown, message)
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(Status::InvalidArgument, message)
    }

    pub fn deadline_exceeded(message: impl Into<String>) -> Self {
        Self::new(Status::DeadlineExceeded, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(Status::NotFound, message)
    }

    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::new(Status::AlreadyExists, message)
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(Status::PermissionDenied, message)
    }

    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::new(Status::ResourceExhausted, message)
    }

    pub fn failed_precondition(message: impl Into<String>) -> Self {
        Self::new(Status::FailedPrecondition, message)
    }

    pub fn aborted(message: impl Into<String>) -> Self {
        Self::new(Status::Aborted, message)
    }

    pub fn out_of_range(message: impl Into<String>) -> Self {
        Self::new(Status::OutOfRange, message)
    }

    pub fn unimplemented(message: impl Into<String>) -> Self {
        Self::new(Status::Unimplemented, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(Status::Internal, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(Status::Unavailable, message)
    }

    pub fn data_loss(message: impl Into<String>) -> Self {
        Self::new(Status::DataLoss, message)
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(Status::Unauthenticated, message)
    }

    pub fn extend(
        code: Status,
        message: impl Into<String>,
        error: impl Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source: Some(error.into()),
        }
    }

    pub fn code(&self) -> Status {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<Status> for Error {
    fn from(code: Status) -> Self {
        Self::new(code, code.description())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;

        let code = match err.kind() {
            ErrorKind::BrokenPipe
            | ErrorKind::WouldBlock
            | ErrorKind::WriteZero
            | ErrorKind::Interrupted => Status::Internal,
            ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::NotConnected
            | ErrorKind::AddrInUse
            | ErrorKind::AddrNotAvailable => Status::Unavailable,
            ErrorKind::AlreadyExists => Status::AlreadyExists,
            ErrorKind::ConnectionAborted => Status::Aborted,
            ErrorKind::InvalidData => Status::DataLoss,
            ErrorKind::InvalidInput => Status::InvalidArgument,
            ErrorKind::NotFound => Status::NotFound,
            ErrorKind::PermissionDenied => Status::PermissionDenied,
            ErrorKind::TimedOut => Status::DeadlineExceeded,
            ErrorKind::UnexpectedEof => Status::OutOfRange,
            _ => Status::Unknown,
        };

        Error::extend(code, err.to_string(), err)
    }
}

impl From<prost::DecodeError> for Error {
    fn from(error: prost::DecodeError) -> Self {
        Self::extend(Status::InvalidArgument, "failed to decode message", error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "status: {:?}, message: {:?}", self.code, self.message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| (&**err) as _)
    }
}
