use crate::{
    error::{Error, ErrorCode},
    protocol::{Notification, Request, RequestId, Response},
    transport::{Message, Transport},
    types::{ClientCapabilities, Implementation, LoggingLevel, LoggingMessage, ServerCapabilities},
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex, RwLock,
};

#[derive(Clone)]
pub struct Session {
    handler: Option<Arc<dyn ClientHandler>>,
    transport: Arc<dyn Transport>,
    receiver: Arc<Mutex<UnboundedReceiver<Message>>>,
    sender: Arc<UnboundedSender<Message>>,
}
impl Session {
    /// Create a new session
    pub fn new(
        transport: Arc<dyn Transport>,
        sender: UnboundedSender<Message>,
        receiver: UnboundedReceiver<Message>,
        handler: Option<Arc<dyn ClientHandler>>,
    ) -> Self {
        Self {
            handler,
            transport,
            sender: Arc::new(sender),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Start the session and listen for messages
    pub async fn start(self) -> Result<(), Error> {
        let transport = self.transport.clone();
        let handler = self.handler.unwrap_or(Arc::new(DefaultClientHandler));
        // listen for messages from the server
        tokio::spawn(async move {
            let mut stream = transport.receive();
            while let Some(result) = stream.next().await {
                match result {
                    Ok(message) => match &message {
                        Message::Request(r) => {
                            let res = handler
                                .handle_request(r.method.clone(), r.params.clone())
                                .await;
                            if transport
                                .send(Message::Response(Response::success(
                                    r.id.clone(),
                                    Some(res.unwrap()),
                                )))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Message::Response(_) => {
                            if self.sender.send(message).is_err() {
                                break;
                            }
                        }
                        Message::Notification(n) => {
                            if handler
                                .handle_notification(n.method.clone(), n.params.clone())
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    },
                    Err(_) => break,
                }
            }
        });
        // listen for requests from the client
        let rx_clone = self.receiver.clone();
        let tx_clone = self.transport.clone();
        tokio::spawn(async move {
            let mut stream = rx_clone.lock().await;
            while let Some(message) = stream.recv().await {
                tx_clone.send(message).await.unwrap();
            }
        });
        Ok(())
    }
}

/// Trait for implementing MCP client handlers
#[async_trait]
pub trait ClientHandler: Send + Sync {
    /// Handle shutdown request
    async fn shutdown(&self) -> Result<(), Error>;

    /// Handle requests
    async fn handle_request(
        &self,
        method: String,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error>;

    /// Handle notifications
    async fn handle_notification(
        &self,
        method: String,
        params: Option<serde_json::Value>,
    ) -> Result<(), Error>;
}

#[derive(Clone, Default)]
pub struct DefaultClientHandler;
#[async_trait]
impl ClientHandler for DefaultClientHandler {
    /// Handle an incoming request
    async fn handle_request(&self, method: String, _params: Option<Value>) -> Result<Value, Error> {
        match method.as_str() {
            "sampling/createMessage" => {
                log::debug!("Got sampling/createMessage");
                Ok(json!({}))
            }
            _ => Err(Error::Other("unknown method".to_string())),
        }
    }

    /// Handle an incoming notification
    async fn handle_notification(
        &self,
        method: String,
        params: Option<Value>,
    ) -> Result<(), Error> {
        match method.as_str() {
            "notifications/message" => {
                // handle logging messages
                if let Some(p) = params {
                    let message: LoggingMessage = serde_json::from_value(p)?;
                    log::log!(message.level.into(), "{}", message.data);
                }
                Ok(())
            }
            "notifications/resources/updated" => {
                if let Some(p) = params {
                    let update_params: HashMap<String, Value> = serde_json::from_value(p)?;
                    if let Some(uri_val) = update_params.get("uri") {
                        let uri = uri_val.as_str().ok_or("some file").unwrap();
                        log::debug!("resource updated: {uri}");
                    }
                }
                Ok(())
            }
            _ => Err(Error::Other("unknown notification".to_string())),
        }
    }

    async fn shutdown(&self) -> Result<(), Error> {
        log::debug!("Client shutting down");
        Ok(())
    }
}

/// MCP client state
#[derive(Clone)]
pub struct Client {
    server_capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    request_counter: Arc<RwLock<i64>>,
    #[allow(dead_code)]
    sender: Arc<UnboundedSender<Message>>,
    receiver: Arc<Mutex<UnboundedReceiver<Message>>>,
}

impl Client {
    /// Create a new MCP client
    pub fn new(sender: UnboundedSender<Message>, receiver: UnboundedReceiver<Message>) -> Self {
        Self {
            server_capabilities: Arc::new(RwLock::new(None)),
            request_counter: Arc::new(RwLock::new(0)),
            sender: Arc::new(sender),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Initialize the client
    pub async fn initialize(
        &self,
        implementation: Implementation,
        capabilities: Option<ClientCapabilities>,
    ) -> Result<ServerCapabilities, Error> {
        let params = serde_json::json!({
            "clientInfo": implementation,
            "capabilities": capabilities.unwrap_or_default(),
            "protocolVersion": crate::LATEST_PROTOCOL_VERSION,
        });
        log::debug!("initializing client with capabilities: {}", params);
        let response = self.request("initialize", Some(params)).await?;
        let mut caps = ServerCapabilities::default();
        if let Some(resp_obj) = response.as_object() {
            if let Some(protocol_version) = resp_obj.get("protocolVersion") {
                if let Some(v) = protocol_version.as_str() {
                    if v != crate::LATEST_PROTOCOL_VERSION {
                        log::warn!(
                            "mcp server does not support the latest protocol version {}",
                            crate::LATEST_PROTOCOL_VERSION
                        );
                        log::warn!("latest supported protocol version is {v}");
                    }
                }
            }
            if let Some(server_caps) = resp_obj.get("capabilities") {
                caps = serde_json::from_value(server_caps.clone())?;
            }
        }
        *self.server_capabilities.write().await = Some(caps.clone());
        // Send initialized notification
        self.notify("initialized", None).await?;
        Ok(caps)
    }

    /// Send a request to the server and wait for the response.
    ///
    /// This method will block until a response is received from the server.
    /// If the server returns an error, it will be propagated as an `Error`.
    pub async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        let mut counter = self.request_counter.write().await;
        *counter += 1;
        let id = RequestId::Number(*counter);

        let request = Request::new(method, params, id.clone());
        self.sender
            .send(Message::Request(request))
            .map_err(|_| Error::Transport("failed to send request message".to_string()))?;
        // Wait for matching response
        let mut receiver = self.receiver.lock().await;
        while let Some(message) = receiver.recv().await {
            if let Message::Response(response) = message {
                if response.id == id {
                    if let Some(error) = response.error {
                        return Err(Error::protocol(
                            match error.code {
                                -32700 => ErrorCode::ParseError,
                                -32600 => ErrorCode::InvalidRequest,
                                -32601 => ErrorCode::MethodNotFound,
                                -32602 => ErrorCode::InvalidParams,
                                -32603 => ErrorCode::InternalError,
                                -32002 => ErrorCode::ServerNotInitialized,
                                -32001 => ErrorCode::UnknownErrorCode,
                                -32000 => ErrorCode::RequestFailed,
                                _ => ErrorCode::UnknownErrorCode,
                            },
                            &error.message,
                        ));
                    }
                    return response.result.ok_or_else(|| {
                        Error::protocol(ErrorCode::InternalError, "Response missing result")
                    });
                }
            }
        }

        Err(Error::protocol(
            ErrorCode::InternalError,
            "Connection closed while waiting for response",
        ))
    }

    pub async fn subscribe(&self, uri: &str) -> Result<(), Error> {
        let mut counter = self.request_counter.write().await;
        *counter += 1;
        let id = RequestId::Number(*counter);

        let request = Request::new("resources/subscribe", Some(json!({"uri": uri})), id.clone());
        self.sender.send(Message::Request(request)).map_err(|_| {
            Error::Transport("failed to send subscribe request message".to_string())
        })?;
        Ok(())
    }

    pub async fn set_log_level(&self, level: LoggingLevel) -> Result<(), Error> {
        let mut counter = self.request_counter.write().await;
        *counter += 1;
        let id = RequestId::Number(*counter);
        let request = Request::new(
            "logging/setLevel",
            Some(json!({"level": level})),
            id.clone(),
        );
        self.sender
            .send(Message::Request(request))
            .map_err(|_| Error::Transport("failed to set logging level".to_string()))?;
        Ok(())
    }

    /// Send a notification to the server
    pub async fn notify(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), Error> {
        let path = format!("notifications/{method}");
        let notification = Notification::new(path, params);
        self.sender
            .send(Message::Notification(notification))
            .map_err(|_| Error::Transport("failed to send notification message".to_string()))
    }

    /// Get the server capabilities
    pub async fn capabilities(&self) -> Option<ServerCapabilities> {
        self.server_capabilities.read().await.clone()
    }

    /// Close the client connection
    pub async fn shutdown(&self) -> Result<(), Error> {
        // Send shutdown request
        self.request("shutdown", None).await?;
        // Send exit notification
        self.notify("exit", None).await?;
        Ok(())
    }
}
