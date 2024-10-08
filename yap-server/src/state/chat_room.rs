use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use warp::ws::{Message};

use super::Client;


/// A seperate, isolated chat space which uers can subscribe and unsubscribe from.
#[derive(Clone, Debug)]
pub struct ChatRoom {
    pub name: String,
    pub clients: Arc<RwLock<HashMap<usize, Client>>>,
}

impl ChatRoom {
    pub fn new(name: String) -> ChatRoom {
        Self {
            name: name,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn subscribe_client(&mut self, client: Client, client_id: usize) {
        self.clients.write().await.insert(client_id, client);
        info!("Client {:#?} subscribed to chat room {:#?}", client_id, self.name);
    }
    pub async fn unsubscribe_client(&mut self, id: usize) {
        self.clients.write().await.remove(&id);
        info!("Client {:#?} unsubscribed to chat room {:#?}", id, self.name);
    }
        /// Broadcasts a message to all connected clients.
        pub async fn broadcast_to_clients(&self, msg: Message) {
            info!("Chat room {:#?} broadcasting message: {:#?}", self.name, &msg);
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
}