//! Standard I/O Transport Implementation
//!
//! This module provides a transport implementation that uses standard input/output (stdio)
//! for communication. This is particularly useful for:
//! - Command-line tools that need to communicate with an MCP server
//! - Local development and testing
//! - Situations where network transport is not desired or available
//!
//! The implementation uses Tokio for asynchronous I/O operations and provides thread-safe
//! access to stdin/stdout through Arc and Mutex.

use async_trait::async_trait;
use futures::Stream;
use std::{pin::Pin, sync::Arc};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};

use crate::{
    error::Error,
    transport::{Message, Transport},
};

pub struct StdioTransport {
    read_connection: Arc<Mutex<Receiver<String>>>,
    write_connection: Sender<String>,
}

impl StdioTransport {
    /// Creates a new stdio transport instance using a child's stdin/stdout.
    pub fn new(read: Receiver<String>, write: Sender<String>) -> Self {
        let transport = Self {
            read_connection: Arc::new(Mutex::new(read)),
            write_connection: write,
        };
        transport
    }
}

#[async_trait]
impl Transport for StdioTransport {
    /// Sends a message by writing it to the child process' stdin
    async fn send(&self, message: Message) -> Result<(), Error> {
        let json = serde_json::to_string(&message)?;
        let _ =
            self.write_connection.send(json).await.map_err(|_| {
                Error::Transport("failed to send message to child process".to_string())
            })?;
        Ok(())
    }

    /// Creates a stream of messages received from the child process' stdout
    ///
    /// # Returns
    ///
    /// Returns a pinned box containing a stream that yields Result<Message, Error>.
    /// The stream will continue until stdin is closed or an error occurs.
    ///
    /// # Message Flow
    ///
    /// 1. Messages are read from stdin in the background task created in `new()`
    /// 2. Each message is sent through the broadcast channel
    /// 3. This stream receives messages from the broadcast channel
    fn receive(&self) -> Pin<Box<dyn Stream<Item = Result<Message, Error>> + Send>> {
        let read_connection = self.read_connection.clone();
        Box::pin(futures::stream::unfold(
            read_connection,
            async move |read_connection| {
                let mut guard = read_connection.lock().await;
                loop {
                    match guard.recv().await {
                        Some(s) => {
                            let message: Message = serde_json::from_str(s.as_str()).unwrap();
                            return Some((Ok(message), read_connection.clone()));
                        }
                        None => return None,
                    }
                }
            },
        ))
    }

    /// Closes the transport.
    ///
    /// For stdio transport, this is a no-op as we don't own stdin/stdout.
    ///
    /// # Returns
    ///
    /// Always returns `Ok(())`.
    async fn close(&self) -> Result<(), Error> {
        // Nothing to do for stdio transport
        Ok(())
    }
}
