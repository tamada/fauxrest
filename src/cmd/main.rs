use clap::{Parser, Subcommand};
use prest::{run, Config};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "prest", version, about = "Static API Generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compiles source JSON data into structured static JSON endpoints.
    Build {
        /// Path to the input data directory.
        inputs: String,
        /// Path to the configuration file.
        #[arg(default_value = "prest.json")]
        config: String,
    },
    /// Starts a lightweight local development server.
    Serve {
        /// Path to the input data directory.
        inputs: String,
        /// Path to the configuration file.
        #[arg(default_value = "prest.json")]
        config: String,
        /// Port to listen on.
        #[arg(short = 'p', long = "port", default_value = "8080")]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build { inputs, config } => {
            let config = Config::load(Path::new(&config)).expect("Failed to load config");
            if let Err(e) = run(config, PathBuf::from(inputs)) {
                eprintln!("Fatal Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Serve { inputs, config, port } => {
            // Note: The original 'serve' functionality needs to be implemented.
            // For now, it's a placeholder as requested by the architecture.
            println!("Starting server on port {} with inputs {} and config {}", port, inputs, config);
        }
    }
}
