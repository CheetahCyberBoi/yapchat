use color_eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::state::globals;
// Some logging globals.
lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", globals::PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}-{}.log", env!("CARGO_PKG_NAME"), chrono::Utc::now().format("%F-%H-%M-%S").to_string());
}

/// Initializes `tracing` and creates necessary folders.
/// **NOTE**: The server expects that the `YAP_SERVER_DATA` env var is set to a valid folder path.
/// # Panics
/// This function will panic if it cannot find the data folder environment variable.
pub fn init() -> Result<()> {
    let directory = globals::DATA_FOLDER.clone().expect("Data folder not found");
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(LOG_FILE.clone());
    let log_file = std::fs::File::create(log_path)?;
    let env_filter = EnvFilter::builder().with_default_directive(tracing::Level::INFO.into());
    // When we're compiling for release we want proper logging
    #[cfg(not(debug_assertions))]
    {
        // If the `RUST_LOG` envar is set, then use that, otherwise
        // use the value of `LOG_ENV`. If that contains errors, this will error.
        let env_filter = env_filter
            .try_from_env()
            .or_else(|_| env_filter.with_env_var(LOG_ENV.clone()).from_env())?;

        let file_subscriber = fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_writer(log_file)
            .with_target(false)
            .with_ansi(false)
            .with_filter(env_filter);
        

        tracing_subscriber::registry()
            .with(file_subscriber)
            .with(ErrorLayer::default())
            .try_init()?;
        
    }
    // Meanwhile for debugs we can hook into `tokio-console` for really good debugging
    #[cfg(debug_assertions)]
    console_subscriber::init();
    Ok(())
}