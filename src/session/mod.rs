use crate::{
    client::{ClientHandler, DefaultClientHandler},
    error::Error,
    protocol::Response,
    transport::{stdio::StdioTransport, Message, Transport},
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::{
    process::Command,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

// #[derive(Clone)]
pub enum Session {
    Local {
        handler: Option<Arc<dyn ClientHandler>>,
        command: Command,
        receiver: Arc<Mutex<UnboundedReceiver<Message>>>,
        sender: Arc<UnboundedSender<Message>>,
    },
    Remote {
        handler: Option<Arc<dyn ClientHandler>>,
        transport: Arc<dyn Transport>,
        receiver: Arc<Mutex<UnboundedReceiver<Message>>>,
        sender: Arc<UnboundedSender<Message>>,
    },
}
impl Session {
    /// Start the session and listen for messages
    pub async fn start(self) -> Result<(), Error> {
        match self {
            Session::Local {
                handler,
                command,
                // receive requests
                receiver,
                // send back responses
                sender,
            } => {
                // spawn the child process
                let mut pm = crate::process::ProcessManager::new();
                let (output_tx, output_rx) = tokio::sync::mpsc::channel(100);
                let process_tx = pm
                    .start_process(command, output_tx.clone())
                    .await
                    .expect("a spawned subprocess");

                let transport = Arc::new(StdioTransport::new(output_rx, process_tx));
                let handler = handler.unwrap_or(Arc::new(DefaultClientHandler));
                let t = transport.clone();

                // listen for messages from the server
                tokio::spawn(async move {
                    let mut stream = t.receive();
                    log::info!("listening for messages from the server");
                    while let Some(result) = stream.next().await {
                        log::info!("got message: {:#?}", result);
                        match result {
                            Ok(message) => match &message {
                                Message::Request(r) => {
                                    let res = handler
                                        .handle_request(r.method.clone(), r.params.clone())
                                        .await;
                                    if t.send(Message::Response(Response::success(
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
                                    if sender.send(message).is_err() {
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
                // listen for messages to send to the server
                let rx_clone = receiver.clone();
                let tx_clone = transport.clone();
                tokio::spawn(async move {
                    let mut stream = rx_clone.lock().await;
                    while let Some(message) = stream.recv().await {
                        log::info!("sending message to server: {:#?}", message);
                        tx_clone.send(message).await.unwrap();
                    }
                });
                Ok(())
            }
            Session::Remote {
                handler,
                transport,
                receiver,
                sender,
            } => {
                let t = transport.clone();
                let handler = handler.unwrap_or(Arc::new(DefaultClientHandler));
                // listen for messages from the server
                tokio::spawn(async move {
                    let mut stream = t.receive();
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(message) => match &message {
                                Message::Request(r) => {
                                    let res = handler
                                        .handle_request(r.method.clone(), r.params.clone())
                                        .await;
                                    if t.send(Message::Response(Response::success(
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
                                    if sender.send(message).is_err() {
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
                // listen for messages to send to the server
                let rx_clone = receiver.clone();
                let tx_clone = transport.clone();
                tokio::spawn(async move {
                    let mut stream = rx_clone.lock().await;
                    while let Some(message) = stream.recv().await {
                        tx_clone.send(message).await.unwrap();
                    }
                });
                Ok(())
            }
        }
    }
}
