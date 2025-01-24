//! Error types for the MCP protocol.
//!
//! This module defines the error types and codes used in the MCP protocol.
//! The error codes follow the JSON-RPC 2.0 specification with some additional
//! MCP-specific error codes.

use std::fmt;
use thiserror::Error;

/// Error codes as defined in the MCP protocol.
///
/// These error codes are based on the JSON-RPC 2.0 specification with additional
/// MCP-specific error codes in the -32000 to -32099 range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// Invalid JSON was received by the server.
    /// An error occurred on the server while parsing the JSON text.
    ParseError = -32700,

    /// The JSON sent is not a valid Request object.
    InvalidRequest = -32600,

    /// The method does not exist / is not available.
    MethodNotFound = -32601,

    /// Invalid method parameter(s).
    InvalidParams = -32602,

    /// Internal JSON-RPC error.
    InternalError = -32603,

    /// Server has not been initialized.
    /// This error is returned when a request is made before the server
    /// has been properly initialized.
    ServerNotInitialized = -32002,

    /// Unknown error code.
    /// This error is returned when an error code is received that is not
    /// recognized by the implementation.
    UnknownErrorCode = -32001,

    /// Request failed.
    /// This error is returned when a request fails for a reason not covered
    /// by other error codes.
    RequestFailed = -32000,
}

impl From<i32> for ErrorCode {
    fn from(code: i32) -> Self {
        match code {
            -32700 => ErrorCode::ParseError,
            -32600 => ErrorCode::InvalidRequest,
            -32601 => ErrorCode::MethodNotFound,
            -32602 => ErrorCode::InvalidParams,
            -32603 => ErrorCode::InternalError,
            -32002 => ErrorCode::ServerNotInitialized,
            -32001 => ErrorCode::UnknownErrorCode,
            -32000 => ErrorCode::RequestFailed,
            _ => ErrorCode::UnknownErrorCode,
        }
    }
}

impl From<ErrorCode> for i32 {
    fn from(code: ErrorCode) -> Self {
        code as i32
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Main error type for MCP operations
#[derive(Error, Debug, Clone)]
pub enum Error {
    /// Protocol error with code and message
    #[error("MCP protocol error: {code:?} - {message}")]
    Protocol {
        /// The error code
        code: ErrorCode,
        /// A message providing more details about the error
        message: String,
        /// Optional additional data about the error
        data: Option<serde_json::Value>,
    },

    /// Transport-related errors
    #[error("Transport error: {0}")]
    Transport(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO-related errors
    #[error("IO error: {0}")]
    Io(String),

    /// Other miscellaneous errors
    #[error("Other error: {0}")]
    Other(String),
}

impl Error {
    /// Create a new protocol error.
    ///
    /// # Arguments
    ///
    /// * `code` - The error code
    /// * `message` - A message describing the error
    pub fn protocol(code: ErrorCode, message: impl Into<String>) -> Self {
        Error::Protocol {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new error with additional data.
    ///
    /// # Arguments
    ///
    /// * `data` - Additional data to attach to the error
    pub fn with_data(self, data: serde_json::Value) -> Self {
        match self {
            Error::Protocol {
                code,
                message,
                data: _,
            } => Error::Protocol {
                code,
                message,
                data: Some(data),
            },
            _ => self,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}
