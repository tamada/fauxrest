mod cmd;

/// Main entry point for the `prest` executable.
///
/// This delegates execution to the CLI command runner.
fn main() {
    if let Err(e) = cmd::run() {
        eprintln!("Fatal Error: {}", e);
        std::process::exit(1);
    }
}
