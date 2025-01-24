use mcp_sdk_rs::{
    client::{Client, Session},
    transport::websocket::WebSocketTransport,
    types::Implementation,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create WebSocket transport
    let transport = WebSocketTransport::new("ws://127.0.0.1:8780").await?;

    // Create mpsc communication channels
    let (request_tx, response_rx) = tokio::sync::mpsc::unbounded_channel();
    let (response_tx, request_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create a session and start listening for requests and notifications
    let session = Session::new(Arc::new(transport), response_tx, request_rx, None);
    session.start().await?;

    // Create MCP client
    let client = Client::new(request_tx, response_rx);

    // Initialize client
    let implementation = Implementation {
        name: "example-client".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    log::debug!("Initializing MCP client...");
    let server_capabilities = client.initialize(implementation, None).await?;
    log::debug!("Server capabilities: {:?}", server_capabilities);

    // Send a request
    log::debug!("Sending test request...");
    let response = client
        .request(
            "test/method",
            Some(serde_json::json!({
                "message": "Hello from Rust MCP client!"
            })),
        )
        .await?;
    log::debug!("Received response: {:?}", response);

    // Shutdown client
    log::debug!("Shutting down...");
    client.shutdown().await?;

    Ok(())
}
