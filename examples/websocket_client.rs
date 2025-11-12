use mcp_sdk_rs::{
    session::Session,
    transport::websocket::WebSocketTransport,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create WebSocket transport
    let transport = WebSocketTransport::new("ws://127.0.0.1:8780").await?;

    // Create mpsc communication channels
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

    // Create a remote session
    let session = Session::Remote {
        handler: None,
        transport: Arc::new(transport) as Arc<dyn mcp_sdk_rs::transport::Transport>,
        receiver: Arc::new(Mutex::new(receiver)),
        sender: Arc::new(sender),
    };

    // Start the session
    session.start().await?;

    println!("WebSocket client session started successfully!");

    Ok(())
}
