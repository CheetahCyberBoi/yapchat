pub mod globals;

use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc, mpsc::UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{warn, info};
use warp::ws::{Message, WebSocket};
use warp::Error;

// Used to add onto the client ID.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// The server's overall state. Stores the clients that are connected alongside the broadcast channel.
#[derive(Clone, Debug)]
pub struct AppState {
    pub clients: Arc<RwLock<HashMap<usize, Client>>>,
    pub message_tx: broadcast::Sender<Message>,
}

impl AppState {
    /// Instantiates a new server, with fresh transmitters and client hashmaps.
    pub fn new() -> Self {
        // FIXME: This should be an unlimited channel at some point, or at least be at like maximum usize or something.
        let (message_tx, _message_rx) = broadcast::channel(100);
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx: message_tx,
        }
    }
    /// Sets up a new client after it has connected via Websockets.
    pub async fn handle_new_client(self: Arc<Self>, ws: WebSocket) {
        // Split the client's channel.
        let (ws_tx, mut ws_rx) = ws.split();
        // Set up all the parameters related to the client
        let (tx, rx) = mpsc::unbounded_channel();
        // This task runs in the background and forwards anything sent by the central server down the channel to the client's Websocket.
        tokio::task::spawn(async {
            let rxs = UnboundedReceiverStream::new(rx);
            rxs.forward(ws_tx).await;
        });
        let new_client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        // Wait for the first message from client to get their username.
        let username = if let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    let json: serde_json::Value = serde_json::from_str(msg.to_str().unwrap_or("{\"usr\": \"Unknown\"}")).unwrap();
                    json["usr"].to_string()
                },
                Err(_) => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        };

        // Broadcast that a new user has joined
        let join_message = json!({
            "usr": "Server",
            "msg": format!("{} has joined the chat", username),
            "timestamp": chrono::Utc::now()
        });
        self.broadcast_to_clients(Message::text(join_message.to_string())).await;
        // Add the new client
        self.clients.write().await.insert(new_client_id, Client::new(tx));
        // Sit and wait for the client to send messages.
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
        // Client no longer sending messages, we can disconnect them.
        //Broadcast that the client has left
        self.close_client(new_client_id).await;
        let leave_message = json!({
            "usr": "Server",
            "msg": format!("{} has left the chat", username),
            "timestamp": chrono::Utc::now()
        });
        self.broadcast_to_clients(Message::text(leave_message.to_string())).await;
        info!("client {} disconnected", new_client_id);
    }
    /// Broadcasts a message to all connected clients.
    pub async fn broadcast_to_clients(&self, msg: Message) {
        info!("Broadcasting message: {:#?}", &msg);
        // Parse the message to a string for ease of sending.
        let msg = if let Ok(msg) = msg.to_str() {
            msg
        } else {
            return;
        };
        // Tack on the timestamp to the message.
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
    /// Disconnects a client entirely.
    pub async fn close_client(&self, id: usize) {
        self.clients.write().await.remove(&id);
    }
}

/// A representation of a client. Currently just a sender, might change to a type alias eventually.
#[derive(Clone, Debug)]
pub struct Client {
    pub sender: UnboundedSender<Result<Message, Error>>,
}

impl Client {
    /// Creates a new client given its `sender`.
    pub fn new(sender: UnboundedSender<Result<Message, Error>>) -> Self {
        Self { sender }
    }
}