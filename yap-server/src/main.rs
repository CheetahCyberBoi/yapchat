mod errors;
mod state;
mod logging;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use warp::Filter;
use warp::ws::{WebSocket};


#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize the logging and errors
    errors::init()?;
    logging::init()?;
    tracing::info!("All set up!");

    // Wrap AppState in an Arc so that it can be shared across requests
    let app_state = Arc::new(state::AppState::new());

    // Clone the Arc<AppState> for use in each request
    let clients = warp::any().map({
        let app_state = app_state.clone();
        move || app_state.clone()  // Clone the Arc on each request
    });
    // Start Warp.
    let route = warp::path::end()
        .and(warp::ws())
        .and(clients)
        .map(|socket: warp::ws::Ws, app_state: Arc<state::AppState>| {
            let app_state_clone = app_state.clone();
            socket.on_upgrade(move |ws| { tracing::info!("Socket connected!"); app_state.handle_new_client(ws)})
        });

    warp::serve(route).run(([0, 0, 0, 0], 80)).await;

    Ok(())
}



