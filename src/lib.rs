//! # Model Context Protocol (MCP) Rust SDK
//!
//! This SDK provides a Rust implementation of the Model Context Protocol (MCP), a protocol designed
//! for communication between AI models and their runtime environments. The SDK supports both client
//! and server implementations with multiple transport layers.
//!
//! ## Features
//!
//! - Full implementation of MCP protocol specification
//! - Multiple transport layers (WebSocket, stdio)
//! - Async/await support using Tokio
//! - Type-safe message handling
//! - Comprehensive error handling
//!
//! ## Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use mcp_rust_sdk::client::Client;
//! use mcp_rust_sdk::transport::websocket::WebSocketTransport;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a WebSocket transport
//!     let transport = WebSocketTransport::new("ws://localhost:8080").await?;
//!
//!     // Create the client with Arc-wrapped transport
//!     let client = Client::new(Arc::new(transport));
//!
//!     // Use the client...
//!
//!     Ok(())
//! }
//! ```

/// Client module provides the MCP client implementation
pub mod client;
/// Error types and handling for the SDK
pub mod error;
/// Protocol-specific types and implementations
pub mod protocol;
/// Server module provides the MCP server implementation
pub mod server;
/// Transport layer implementations (WebSocket, stdio)
pub mod transport;
/// Common types used throughout the SDK
pub mod types;

// Re-export commonly used types for convenience
pub use error::Error;
pub use protocol::{
    Notification, Request, Response, JSONRPC_VERSION, LATEST_PROTOCOL_VERSION,
    SUPPORTED_PROTOCOL_VERSIONS,
};
pub use types::*;
