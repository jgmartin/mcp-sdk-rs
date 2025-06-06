use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

use crate::error::{Error, ErrorCode};

/// The latest supported protocol version of MCP
///
/// This version represents the most recent protocol specification that this SDK supports.
/// It is used during client-server handshake to ensure compatibility.
pub const LATEST_PROTOCOL_VERSION: &str = "2025-03-26";

/// List of all protocol versions supported by this SDK
///
/// This list is used during version negotiation to determine compatibility between
/// client and server. The versions are listed in order of preference, with the
/// most recent version first.
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] =
    &[LATEST_PROTOCOL_VERSION, "2024-11-05", "2024-10-07"];

/// JSON-RPC version used by the MCP protocol
///
/// MCP uses JSON-RPC 2.0 for its message format. This constant is used to ensure
/// all messages conform to the correct specification.
pub const JSONRPC_VERSION: &str = "2.0";

/// A unique identifier for a request
///
/// This enum represents the possible types of request identifiers in the MCP protocol.
/// It can be either a string or a number, as per JSON-RPC 2.0 specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// String representation of the request ID
    String(String),
    /// Numeric representation of the request ID
    Number(i64),
}

/// Base JSON-RPC request structure
///
/// This struct represents a JSON-RPC request in the MCP protocol.
/// It includes the JSON-RPC version, method name, optional parameters, and a unique identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Name of the method to be invoked
    pub method: String,
    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Unique identifier for the request
    pub id: RequestId,
}

/// Base JSON-RPC notification structure
///
/// This struct represents a JSON-RPC notification in the MCP protocol.
/// It is similar to a request but does not include an id field, as it does not expect a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Name of the method to be invoked
    pub method: String,
    /// Optional parameters for the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Base JSON-RPC response structure
///
/// This struct represents a JSON-RPC response in the MCP protocol.
/// It includes the JSON-RPC version, the id of the request it's responding to,
/// and either a result or an error field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// ID of the request this response corresponds to
    pub id: RequestId,
    /// The result of a successful request (null if there was an error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// The error object if the request failed (null if the request was successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
}

/// JSON-RPC error object
///
/// This struct represents an error that occurred during the processing of a JSON-RPC request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    /// The error code
    pub code: i32,
    /// A short description of the error
    pub message: String,
    /// Additional information about the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl Request {
    /// Creates a new Request instance
    ///
    /// # Arguments
    ///
    /// * `method` - The name of the method to be invoked
    /// * `params` - Optional parameters for the method
    /// * `id` - Unique identifier for the request
    ///
    /// # Returns
    ///
    /// A new Request instance
    pub fn new(method: impl Into<String>, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: crate::JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id,
        }
    }
}

impl Notification {
    /// Creates a new Notification instance
    ///
    /// # Arguments
    ///
    /// * `method` - The name of the method to be invoked
    /// * `params` - Optional parameters for the method
    ///
    /// # Returns
    ///
    /// A new Notification instance
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: crate::JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

impl Response {
    /// Creates a new successful Response instance
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the request this response corresponds to
    /// * `result` - The result of the successful request
    ///
    /// # Returns
    ///
    /// A new Response instance representing a successful result
    pub fn success(id: RequestId, result: Option<Value>) -> Self {
        Self {
            jsonrpc: crate::JSONRPC_VERSION.to_string(),
            id,
            result,
            error: None,
        }
    }

    /// Creates a new error Response instance
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the request this response corresponds to
    /// * `error` - The error that occurred during request processing
    ///
    /// # Returns
    ///
    /// A new Response instance representing an error
    pub fn error(id: RequestId, error: ResponseError) -> Self {
        Self {
            jsonrpc: crate::JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl From<Error> for ResponseError {
    fn from(err: Error) -> Self {
        match err {
            Error::Protocol {
                code,
                message,
                data,
            } => ResponseError {
                code: code.into(),
                message,
                data,
            },
            Error::Transport(msg) => ResponseError {
                code: ErrorCode::InternalError.into(),
                message: format!("Transport error: {}", msg),
                data: None,
            },
            Error::Serialization(err) => ResponseError {
                code: ErrorCode::ParseError.into(),
                message: err.to_string(),
                data: None,
            },
            Error::Io(err) => ResponseError {
                code: ErrorCode::InternalError.into(),
                message: err.to_string(),
                data: None,
            },
            Error::Other(msg) => ResponseError {
                code: ErrorCode::InternalError.into(),
                message: msg,
                data: None,
            },
        }
    }
}

impl fmt::Display for RequestId {
    /// Provides a string representation of the RequestId
    ///
    /// This implementation allows RequestId to be easily printed or converted to a string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_creation() {
        let id = RequestId::Number(1);
        let params = Some(json!({"key": "value"}));
        let request = Request::new("test_method", params.clone(), id.clone());

        assert_eq!(request.jsonrpc, JSONRPC_VERSION);
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, params);
        assert_eq!(request.id, id);
    }

    #[test]
    fn test_notification_creation() {
        let params = Some(json!({"event": "update"}));
        let notification = Notification::new("test_event", params.clone());

        assert_eq!(notification.jsonrpc, JSONRPC_VERSION);
        assert_eq!(notification.method, "test_event");
        assert_eq!(notification.params, params);
    }

    #[test]
    fn test_response_success() {
        let id = RequestId::String("test-1".to_string());
        let result = Some(json!({"status": "ok"}));
        let response = Response::success(id.clone(), result.clone());

        assert_eq!(response.jsonrpc, JSONRPC_VERSION);
        assert_eq!(response.id, id);
        assert_eq!(response.result, result);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let id = RequestId::Number(123);
        let error = ResponseError {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: Some(json!({"details": "missing method"})),
        };
        let response = Response::error(id.clone(), error.clone());

        assert_eq!(response.jsonrpc, JSONRPC_VERSION);
        assert_eq!(response.id, id);
        assert!(response.result.is_none());

        let response_error = response.error.unwrap();
        assert_eq!(response_error.code, error.code);
        assert_eq!(response_error.message, error.message);
    }

    #[test]
    fn test_request_id_display() {
        let num_id = RequestId::Number(42);
        let str_id = RequestId::String("test-id".to_string());

        assert_eq!(num_id.to_string(), "42");
        assert_eq!(str_id.to_string(), "test-id");
    }

    #[test]
    fn test_protocol_versions() {
        assert!(SUPPORTED_PROTOCOL_VERSIONS.contains(&LATEST_PROTOCOL_VERSION));
        assert_eq!(JSONRPC_VERSION, "2.0");
    }
}
