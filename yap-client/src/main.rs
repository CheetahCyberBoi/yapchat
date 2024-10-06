use chrono::{DateTime, Local, Utc};
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use serde::Deserialize;
use std::io;
use std::io::Write;

use tokio_tungstenite::connect_async;
use tokio_tungstenite::MaybeTlsStream;


#[tokio::main]
async fn main() {
    console_subscriber::init();
    let mut buffer = String::new();
    let stdin = io::stdin();

    println!("Welcome to Yap Client v0.1");

    print!("Enter your username: ");
    io::stdout().flush().unwrap(); // ensures that the prompt appears immediately.

    stdin.read_line(&mut buffer).unwrap();
    let username = buffer.trim().to_string(); // Get or client's uername. let's trim it as well.

    //Get the ip and the port of the server.
    buffer.clear();
    print!("Enter IP and port of the YapServer (e.g. 127.0.0.1:80): ");
    io::stdout().flush().unwrap(); // ensure that the prompt appears immediately.

    stdin.read_line(&mut buffer).unwrap();

    //Trim the input and ensure that the URI includes WebSocket shcmeme
    let raw_ip = buffer.trim();
    let ws_uri = if raw_ip.starts_with("ws://") || raw_ip.starts_with("wss://") {
        raw_ip.to_string()
    } else {
        format!("wss://{}", raw_ip)
    };

    // Parse the URI and initiate the WebSocket connection.
    println!("Connecting to your YapServer at {}", ws_uri);
    let (ws_stream, _) = connect_async(ws_uri).await.unwrap(); // parse the URI and connect.
    println!("Connected!");
    // Split the WebSocket to handle concurrent use.
    let (mut write_stream, mut read_stream) = ws_stream.split();

    //Spawn a task to handle reading from the WebSocket

    tokio::spawn(async move {
        while let Some(msg) = read_stream.next().await {
            match msg {
                Ok(msg) => {
                    if let Ok(text) = msg.to_text() {
                        let message_from_server: serde_json::Value = serde_json::from_str(text).expect("Failed to parse message.");

                        // Parse UTC timestamp into `chrono::DateTime`
                        let date_time: DateTime<Utc> = message_from_server["timestamp"]
                            .as_str()
                            .unwrap()
                            .parse()
                            .unwrap();
                        // Convert to local time
                        let local_date_time = date_time.with_timezone(&Local);

                        // Convert to H:M format
                        let formatted_time = local_date_time.format("%H:%M").to_string();

                        println!("[{:#}] {}: {}", 
                                formatted_time,
                                message_from_server["usr"], 
                                message_from_server["msg"]);
                    }
                }

                Err(e) => {
                    println!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    loop {
        let mut buffer = String::new();
        let mut message = json!({ "usr": username.clone() });
        
        //println!("Enter your message: "); // don't need, easy solution to the mismatch problem.
        //buffer.clear();
        stdin.read_line(&mut buffer).unwrap();
        io::stdout().flush().unwrap();

        message["msg"] = json!(buffer.trim());

        // Handle exiting
        if buffer.trim() == "/exit" {
            println!("Exiting Yap Client v0.1...");
            break;
        }
        
        // Send the message to the server via WebSocket
        write_stream.send(tungstenite::Message::text(serde_json::to_string(&message).unwrap()))
            .await
            .expect("Failed to send message to server");

    }
}
