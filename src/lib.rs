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
//! use mcp_sdk_rs::client::Client;
//! use mcp_sdk_rs::session::Session;
//! use mcp_sdk_rs::transport::websocket::WebSocketTransport;
//! use tokio::sync::Mutex;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create WebSocket transport
//!     let transport = WebSocketTransport::new("ws://127.0.0.1:8780").await?;
//!
//!     // Create client communication channel
//!     let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
//!
//!     // Create a session and start listening for requests and notifications
//!     let session = Session::Remote {
//!         handler: None,
//!         transport: Arc::new(transport) as Arc<dyn mcp_sdk_rs::transport::Transport>,
//!         receiver: Arc::new(Mutex::new(receiver)),
//!         sender: Arc::new(sender),
//!     };
//!     session.start().await?;
//!
//!     Ok(())
//! }
//! ```

/// Client module provides the MCP client implementation
pub mod client;
/// Error types and handling for the SDK
pub mod error;
/// Process management for local MCP servers
pub mod process;
/// Protocol-specific types and implementations
pub mod protocol;
/// Server module provides the MCP server implementation
pub mod server;
pub mod session;
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
