pub mod globals;
pub mod chat_room;
pub mod client;

use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{warn, info};
use warp::ws::{Message, WebSocket};

use self::client::Client;
use self::chat_room::ChatRoom;

// Used to add onto the client ID.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// The server's overall state. Stores the clients that are connected alongside the broadcast channel.
#[derive(Clone, Debug)]
pub struct AppState {
    pub clients: Arc<RwLock<HashMap<usize, Client>>>,
    pub chat_rooms: Arc<RwLock<HashMap<usize, ChatRoom>>>,
    pub message_tx: broadcast::Sender<Message>,
}

impl AppState {
    /// Instantiates a new server, with fresh transmitters and client hashmaps.
    pub async fn new() -> Self {
        // FIXME: This should be an unlimited channel at some point, or at least be at like maximum usize or something.
        let (message_tx, _message_rx) = broadcast::channel(100);
        let chat_rooms = Arc::new(RwLock::new(HashMap::new()));
        chat_rooms.write().await.insert(0, ChatRoom::new("main".to_string()));
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            chat_rooms: chat_rooms,
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
            "msg": format!("{} has joined the server", username),
            "timestamp": chrono::Utc::now()
        });
        self.broadcast_to_rooms(Message::text(join_message.to_string())).await;
        // Add the new client
        self.clients.write().await.insert(new_client_id, Client::new(tx, self.chat_rooms.read().await.get(&0).expect("Default room not created!").clone()));
        // Sit and wait for the client to send messages.
        while let Some(r) = ws_rx.next().await {
            let msg = match r {
                Ok(msg) => msg,
                Err(e) =>  {
                    warn!("Error receiving message from client: {:?}", e);
                    break;
                },
            };
            self.broadcast_to_rooms(msg).await;
        }
        // Client no longer sending messages, we can disconnect them.
        //Broadcast that the client has left
        self.close_client(new_client_id).await;
        let leave_message = json!({
            "usr": "Server",
            "msg": format!("{} has left the server", username),
            "timestamp": chrono::Utc::now()
        });
        self.broadcast_to_rooms(Message::text(leave_message.to_string())).await;
        info!("client {} disconnected", new_client_id);
    }
    /// Broadcasts a message to all the chat rooms.
    pub async fn broadcast_to_rooms(&self, msg: Message) {
        // Read lock to access rooms.
        let rooms = self.chat_rooms.read().await;

        for (room_id, room) in rooms.iter() {
            // Clone the message each time
            let msg_clone = msg.clone();
            

            //Broadcast to the room.
            room.broadcast_to_clients(msg_clone).await;
            info!("Room id {:#?} is broadcasting a message", room_id);
        }
    }
    /// Disconnects a client entirely.
    pub async fn close_client(&self, id: usize) {
        self.clients.write().await.remove(&id);
    }
    /// Moves a client to a room, creating said room if it  does not exist yet
    pub async fn move_client_to_room(&self, client_id: usize, room_id: usize, room_name: String) {
        // Set up chatroom
        let next_room_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
        let maybe_room = self.get_room(room_id).await;
        let mut chat_room = match maybe_room {
            Some(room) => room,
            None => {
                let room = ChatRoom::new(room_name);
                room.clients.write().await.insert(0, self.clients.read().await.get(&client_id).unwrap().clone());
                self.chat_rooms.write().await.insert(next_room_id, room);
                self.get_room(room_id).await.unwrap()
            }
        };
        
        let mut old_room = self.get_client(client_id).await.room;
        old_room.unsubscribe_client(client_id).await;
        chat_room.subscribe_client(self.clients.read().await.get(&client_id).unwrap().clone(), client_id).await;
        info!("Client {:#?} moved from room {:#?} to room {:#?}", &client_id, old_room.name, chat_room.name);
    }
    /// Utility function: gets a client from it's id
    pub async fn get_client(&self, client_id: usize) -> Client {
        self.clients.read().await.get(&client_id).unwrap().clone()
    }
    /// Utility function: gets a room from it's id
    pub async fn get_room(&self, room_id: usize) -> Option<ChatRoom> {
        self.chat_rooms.read().await.get(&room_id).cloned()
    }
}