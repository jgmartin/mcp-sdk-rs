# Model Context Protocol (MCP) Rust SDK

> ⚠️ **Warning**: This SDK is currently a work in progress and is not ready for production use.

A Rust implementation of the Model Context Protocol (MCP), designed for seamless communication between AI models and their runtime environments.

## Features

- 🚀 Full implementation of MCP protocol specification
- 🔄 Multiple transport layers (WebSocket, stdio)
- ⚡ Async/await support using Tokio
- 🛡️ Type-safe message handling
- 🔍 Comprehensive error handling
- 📦 Zero-copy serialization/deserialization

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mcp_sdk_rs = "0.1.0"
```

## Quick Start

### Client Example

```rust
use mcp_sdk_rs::{Client, transport::WebSocketTransport};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Create a transport
    let transport = WebSocketTransport::new(self.url.as_str())
        .await
        .map_err(|_| Error::Internal)?;

    //Create mpsc channels for communication between the client and session
    let (request_tx, request_rx): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
        tokio::sync::mpsc::unbounded_channel();
    let (response_tx, response_rx): (UnboundedSender<Message>, UnboundedReceiver<Message>) =
        tokio::sync::mpsc::unbounded_channel();

    // Create and start the session
    // Optionally pass an implementation of ClientHandler for custom handling of requests and notifications
    let session = Session::new(Arc::new(transport), response_tx, request_rx, None);
    session.start().await.map_err(|_| Error::Internal)?;

    // Create the client and make requests, receive notifications etc
    let client = Client::new(request_tx, response_rx);
    let response = client.request(
        "tools/call",
        Some(json!({
            "name": "methondName",
            "arguments": json!({})
        })),
    )
    .await?
}
```

### Server Example

```rust
use mcp_sdk_rs::{Server, transport::StdioTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a stdio transport
    let (transport, _) = StdioTransport::new();

    // Create and start the server
    let server = Server::new(transport);
    server.start().await?;

    Ok(())
}
```

## Transport Layers

The SDK supports multiple transport layers:

### WebSocket Transport
- Ideal for network-based communication
- Supports both secure (WSS) and standard (WS) connections
- Built-in reconnection handling

### stdio Transport
- Perfect for local process communication
- Lightweight and efficient
- Great for command-line tools and local development

## Error Handling

The SDK provides comprehensive error handling through the `Error` type:

```rust
use mcp_sdk_rs::Error;

match result {
    Ok(value) => println!("Success: {:?}", value),
    Err(Error::Protocol(code, msg)) => println!("Protocol error {}: {}", code, msg),
    Err(Error::Transport(e)) => println!("Transport error: {}", e),
    Err(e) => println!("Other error: {}", e),
}
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Originally forked from [https://github.com/Derek-X-Wang/mcp-rust-sdk](mcp-rust-sdk).
