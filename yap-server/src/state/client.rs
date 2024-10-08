use tokio::sync::mpsc::UnboundedSender;
use warp::ws::Message;
use warp::Error;

use super::chat_room::ChatRoom;

/// A representation of a client. Currently just a sender, might change to a type alias eventually.
#[derive(Clone, Debug)]
pub struct Client {
    pub sender: UnboundedSender<Result<Message, Error>>,
    pub room: ChatRoom,

}

impl Client {
    /// Creates a new client given its `sender`.
    pub fn new(sender: UnboundedSender<Result<Message, Error>>, chat_room: ChatRoom) -> Self {
        Self { sender, room: chat_room }
    }
}