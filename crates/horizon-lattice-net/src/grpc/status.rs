//! gRPC status codes and errors.

use std::fmt;

/// gRPC status codes.
///
/// These correspond to the standard gRPC status codes defined in
/// the gRPC specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GrpcStatusCode {
    /// The operation completed successfully.
    Ok = 0,
    /// The operation was cancelled (typically by the caller).
    Cancelled = 1,
    /// Unknown error.
    Unknown = 2,
    /// Invalid argument was provided.
    InvalidArgument = 3,
    /// Deadline expired before operation could complete.
    DeadlineExceeded = 4,
    /// Requested entity was not found.
    NotFound = 5,
    /// Entity already exists.
    AlreadyExists = 6,
    /// Permission denied.
    PermissionDenied = 7,
    /// Resource exhausted (e.g., rate limit exceeded).
    ResourceExhausted = 8,
    /// Precondition failed.
    FailedPrecondition = 9,
    /// Operation was aborted.
    Aborted = 10,
    /// Operation was out of valid range.
    OutOfRange = 11,
    /// Operation is not implemented.
    Unimplemented = 12,
    /// Internal error.
    Internal = 13,
    /// Service is unavailable.
    Unavailable = 14,
    /// Data loss occurred.
    DataLoss = 15,
    /// Unauthenticated request.
    Unauthenticated = 16,
}

impl GrpcStatusCode {
    /// Create from an i32 code.
    pub fn from_i32(code: i32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::Cancelled,
            2 => Self::Unknown,
            3 => Self::InvalidArgument,
            4 => Self::DeadlineExceeded,
            5 => Self::NotFound,
            6 => Self::AlreadyExists,
            7 => Self::PermissionDenied,
            8 => Self::ResourceExhausted,
            9 => Self::FailedPrecondition,
            10 => Self::Aborted,
            11 => Self::OutOfRange,
            12 => Self::Unimplemented,
            13 => Self::Internal,
            14 => Self::Unavailable,
            15 => Self::DataLoss,
            16 => Self::Unauthenticated,
            _ => Self::Unknown,
        }
    }

    /// Get the i32 value of this code.
    pub fn to_i32(self) -> i32 {
        self as i32
    }

    /// Check if this is an OK status.
    pub fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Check if this is an error status.
    pub fn is_error(self) -> bool {
        !self.is_ok()
    }

    /// Get a human-readable description of the status code.
    pub fn description(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Cancelled => "Cancelled",
            Self::Unknown => "Unknown",
            Self::InvalidArgument => "Invalid Argument",
            Self::DeadlineExceeded => "Deadline Exceeded",
            Self::NotFound => "Not Found",
            Self::AlreadyExists => "Already Exists",
            Self::PermissionDenied => "Permission Denied",
            Self::ResourceExhausted => "Resource Exhausted",
            Self::FailedPrecondition => "Failed Precondition",
            Self::Aborted => "Aborted",
            Self::OutOfRange => "Out of Range",
            Self::Unimplemented => "Unimplemented",
            Self::Internal => "Internal",
            Self::Unavailable => "Unavailable",
            Self::DataLoss => "Data Loss",
            Self::Unauthenticated => "Unauthenticated",
        }
    }
}

impl fmt::Display for GrpcStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl From<tonic::Code> for GrpcStatusCode {
    fn from(code: tonic::Code) -> Self {
        Self::from_i32(code as i32)
    }
}

impl From<GrpcStatusCode> for tonic::Code {
    fn from(code: GrpcStatusCode) -> Self {
        match code {
            GrpcStatusCode::Ok => tonic::Code::Ok,
            GrpcStatusCode::Cancelled => tonic::Code::Cancelled,
            GrpcStatusCode::Unknown => tonic::Code::Unknown,
            GrpcStatusCode::InvalidArgument => tonic::Code::InvalidArgument,
            GrpcStatusCode::DeadlineExceeded => tonic::Code::DeadlineExceeded,
            GrpcStatusCode::NotFound => tonic::Code::NotFound,
            GrpcStatusCode::AlreadyExists => tonic::Code::AlreadyExists,
            GrpcStatusCode::PermissionDenied => tonic::Code::PermissionDenied,
            GrpcStatusCode::ResourceExhausted => tonic::Code::ResourceExhausted,
            GrpcStatusCode::FailedPrecondition => tonic::Code::FailedPrecondition,
            GrpcStatusCode::Aborted => tonic::Code::Aborted,
            GrpcStatusCode::OutOfRange => tonic::Code::OutOfRange,
            GrpcStatusCode::Unimplemented => tonic::Code::Unimplemented,
            GrpcStatusCode::Internal => tonic::Code::Internal,
            GrpcStatusCode::Unavailable => tonic::Code::Unavailable,
            GrpcStatusCode::DataLoss => tonic::Code::DataLoss,
            GrpcStatusCode::Unauthenticated => tonic::Code::Unauthenticated,
        }
    }
}

/// A gRPC status representing the result of an RPC call.
#[derive(Debug, Clone)]
pub struct GrpcStatus {
    /// The status code.
    pub code: GrpcStatusCode,
    /// An optional error message.
    pub message: String,
    /// Optional error details (serialized protobuf).
    pub details: Option<Vec<u8>>,
}

impl GrpcStatus {
    /// Create a new status.
    pub fn new(code: GrpcStatusCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Create an OK status.
    pub fn ok() -> Self {
        Self::new(GrpcStatusCode::Ok, "")
    }

    /// Create a cancelled status.
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::Cancelled, message)
    }

    /// Create an invalid argument status.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::InvalidArgument, message)
    }

    /// Create a not found status.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::NotFound, message)
    }

    /// Create a permission denied status.
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::PermissionDenied, message)
    }

    /// Create an internal error status.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::Internal, message)
    }

    /// Create an unavailable status.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::Unavailable, message)
    }

    /// Create an unauthenticated status.
    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::Unauthenticated, message)
    }

    /// Create an unknown error status.
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(GrpcStatusCode::Unknown, message)
    }

    /// Add error details.
    pub fn with_details(mut self, details: Vec<u8>) -> Self {
        self.details = Some(details);
        self
    }

    /// Check if this is an OK status.
    pub fn is_ok(&self) -> bool {
        self.code.is_ok()
    }

    /// Check if this is an error status.
    pub fn is_error(&self) -> bool {
        self.code.is_error()
    }

    /// Convert to a Result.
    pub fn to_result<T>(self, value: T) -> Result<T, Self> {
        if self.is_ok() { Ok(value) } else { Err(self) }
    }
}

impl fmt::Display for GrpcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.message.is_empty() {
            write!(f, "{}", self.code)
        } else {
            write!(f, "{}: {}", self.code, self.message)
        }
    }
}

impl std::error::Error for GrpcStatus {}

impl From<tonic::Status> for GrpcStatus {
    fn from(status: tonic::Status) -> Self {
        Self {
            code: GrpcStatusCode::from(status.code()),
            message: status.message().to_string(),
            details: if status.details().is_empty() {
                None
            } else {
                Some(status.details().to_vec())
            },
        }
    }
}

impl From<GrpcStatus> for tonic::Status {
    fn from(status: GrpcStatus) -> Self {
        let tonic_status = tonic::Status::new(status.code.into(), status.message);
        if let Some(details) = status.details {
            // Note: tonic::Status doesn't have a public method to set details directly
            // The details are typically set through the protobuf any type
            let _ = details;
        }
        tonic_status
    }
}

impl From<GrpcStatus> for crate::error::NetworkError {
    fn from(status: GrpcStatus) -> Self {
        crate::error::NetworkError::Grpc(status.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_from_i32() {
        assert_eq!(GrpcStatusCode::from_i32(0), GrpcStatusCode::Ok);
        assert_eq!(GrpcStatusCode::from_i32(1), GrpcStatusCode::Cancelled);
        assert_eq!(GrpcStatusCode::from_i32(100), GrpcStatusCode::Unknown);
    }

    #[test]
    fn test_status_code_is_ok() {
        assert!(GrpcStatusCode::Ok.is_ok());
        assert!(!GrpcStatusCode::Internal.is_ok());
    }

    #[test]
    fn test_status_creation() {
        let status = GrpcStatus::not_found("User not found");
        assert_eq!(status.code, GrpcStatusCode::NotFound);
        assert_eq!(status.message, "User not found");
        assert!(status.is_error());
    }

    #[test]
    fn test_status_ok() {
        let status = GrpcStatus::ok();
        assert!(status.is_ok());
        assert!(!status.is_error());
    }

    #[test]
    fn test_status_display() {
        let status = GrpcStatus::internal("Something went wrong");
        assert_eq!(status.to_string(), "Internal: Something went wrong");
    }
}
