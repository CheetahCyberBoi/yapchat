mod errors;
mod state;
mod logging;

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use warp::Filter;
use warp::ws::{WebSocket};

use crate::state::AppState;

// Passes the websocket off to the appstate to handle.
async fn handle_websocket(ws: WebSocket, app_state: Arc<AppState>) {
    app_state.handle_new_client(ws).await;
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // Initialize the logging and errors
    errors::init()?;
    logging::init()?;
    tracing::info!("All set up!");

    // Wrap AppState in an Arc so that it can be shared across requests
    let app_state = Arc::new(state::AppState::new().await);

    // Clone the Arc<AppState> for use in each request
    let clients = warp::any().map({
        move || app_state.clone()
    });


    //Add code here.
    let route = warp::path::end()
        .and(warp::ws())
        .and(clients)
        .map(|ws: warp::ws::Ws, app_state: Arc<AppState>| {
            ws.on_upgrade(move |socket| handle_websocket(socket, app_state))
        });




    warp::serve(route).run(([0, 0, 0, 0], 80)).await;

    Ok(())
}


    // Start Warp.
    // let route = warp::path::end()
    //     .and(warp::ws())
    //     .and(clients)
    //     .and_then(|ws: warp::ws::Ws, app_state: Arc<AppState>| async move {
    //         // Clone the appstate to move it into the async block
    //         let app_state_clone = app_state.clone();

    //         ws.on_upgrade(move |socket| { 
    //             tracing::info!("Socket connected!"); 
    //             app_state_clone.handle_new_client(socket)
    //         });

    //         Ok::<_, warp::Rejection>(())
    //     });


