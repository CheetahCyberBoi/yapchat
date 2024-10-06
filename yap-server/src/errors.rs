use color_eyre::Result;
use tracing::error;

/// Initializes the panic hooks and eyre hooks used by this application for fancy error handling.
pub fn init() -> Result<()> {
    // Get the hooks from color_eyre.
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it to Imulat Devs ifwi4@email.com Thank you."
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    // Install the eyre hook so we have fancy looking errors. Mainly for the traces if anything.
    eyre_hook.install()?;
    // Set up the regular panic hooks.
    std::panic::set_hook(Box::new(move |panic_info| {
        
        // If we're compiling for release the end-user should be pretty informed about what to do if the app crashes.
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, metadata, print_msg};
            // Change this as needed.
            let metadata = metadata!()
                .authors("Imulat Devs Inc. <ifwi4@email.com>")
                .homepage("imulat.free.nf")
                .support("Please contact Imulat Support regarding this issue at <ifwi4@email.com>.");
            let file_path = handle_dump(&metadata, panic_info);
            //prints human-panic message
            print_msg(file_path, &metadata)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color_eyre stacktrace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));
        // When we're compiling for debug reasons we really just need a lot of information fast.
        #[cfg(debug_assertions)]
        {
            //Better panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }
        // Exit the process at the end of the hooks.
        std::process::exit(libc::EXIT_FAILURE);
    }));
    Ok(())
}

/// Stole this from Ratatui's component template.
/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
        (target: $target:expr, level: $level:expr, $ex:expr) => {{
                match $ex {
                        value => {
                                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                                value
                        }
                }
        }};
        (level: $level:expr, $ex:expr) => {
                trace_dbg!(target: module_path!(), level: $level, $ex)
        };
        (target: $target:expr, $ex:expr) => {
                trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
        };
        ($ex:expr) => {
                trace_dbg!(level: tracing::Level::DEBUG, $ex)
        };
}