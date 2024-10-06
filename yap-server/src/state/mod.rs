pub mod globals;

use futures::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc, mpsc::UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{warn, info};
use warp::ws::{Message, WebSocket};
use warp::Error;

static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);
#[derive(Clone, Debug)]
pub struct AppState {
    pub clients: Arc<RwLock<HashMap<usize, Client>>>,
    pub message_tx: broadcast::Sender<Message>,
}

impl AppState {
    pub fn new() -> Self {
        let (message_tx, _message_rx) = broadcast::channel(100);
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx: message_tx,
        }
    }

    pub async fn handle_new_client(self: Arc<Self>, ws: WebSocket) {
        let (ws_tx, mut ws_rx) = ws.split();

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::task::spawn(async {
            let rxs = UnboundedReceiverStream::new(rx);
            rxs.forward(ws_tx).await;
        });
        let new_client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        self.clients.write().await.insert(new_client_id, Client::new(tx));
        while let Some(r) = ws_rx.next().await {
            let msg = match r {
                Ok(msg) => msg,
                Err(e) =>  {
                    warn!("Error receiving message from client: {:?}", e);
                    break;
                },
            };
            self.broadcast_to_clients(msg).await;
        }
        self.close_client(new_client_id).await;
        info!("client {} disconnected", new_client_id);
    }

    pub async fn broadcast_to_clients(&self, msg: Message) {
        info!("Broadcasting message: {:#?}", &msg);
        let msg = if let Ok(msg) = msg.to_str() {
            msg
        } else {
            return;
        };
        let mut msg: serde_json::Value = serde_json::from_str(msg).expect("failed to parse message from client");
        msg["timestamp"] = serde_json::json!(chrono::Utc::now());
        info!("Appended message with timestamp");

        //Collect disconnected clients to remove later
        let mut disconnected_clients = Vec::new();

        //Broadcast to all clients
        for (&id, client) in self.clients.read().await.iter() {
            if let Err(_e) = client.sender.send(Ok(Message::text(msg.to_string()))) {
                // Log the error and mark the client for removal
                warn!("Failed to send message to client {}, marking for removal", id);
                disconnected_clients.push(id);
            }
        }

        //Clean up the disconnected clients
        if !disconnected_clients.is_empty() {
            let mut clients = self.clients.write().await;
            for id in disconnected_clients {
                clients.remove(&id);
                info!("Removed client {} due to disconnection", id);
            }
        }
    }

    pub async fn close_client(&self, id: usize) {
        self.clients.write().await.remove(&id);
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    pub sender: UnboundedSender<Result<Message, Error>>,
}

impl Client {
    pub fn new(sender: UnboundedSender<Result<Message, Error>>) -> Self {
        Self { sender }
    }
}