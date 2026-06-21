use clap::{Args, Parser, Subcommand};

/// Command line arguments parser for `prest`.
#[derive(Parser, Debug)]
#[command(name = "prest", version, about = "Static API Generator", long_about = None)]
pub struct Cli {
    /// Subcommand to execute. Defaults to `build` if not provided.
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Global option: Path to the input data directory or a single JSON file.
    #[arg(short = 'i', long = "inputs", default_value = "./data")]
    pub inputs: String,

    /// Global option: Path to the output/destination directory where static files will be generated.
    #[arg(short = 'd', long = "dest", default_value = "./dist")]
    pub dest: String,
}

/// Commands supported by the `prest` CLI.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Compiles source JSON data into structured static JSON endpoints.
    Build(BuildArgs),
    /// Starts a lightweight local development server.
    Serve(ServeArgs),
}

/// Arguments for the `build` subcommand.
#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Path to the input data directory or a single JSON file.
    #[arg(short = 'i', long = "inputs", default_value = "./data")]
    pub inputs: String,

    /// Path to the output/destination directory where static files will be generated.
    #[arg(short = 'd', long = "dest", default_value = "./dist")]
    pub dest: String,
}

/// Arguments for the `serve` subcommand.
#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// Path to the input data directory or a single JSON file.
    #[arg(short = 'i', long = "inputs", default_value = "./data")]
    pub inputs: String,

    /// Port to listen on.
    #[arg(short = 'p', long = "port", default_value = "8080")]
    pub port: u16,

    /// Host/IP address to bind the server to.
    #[arg(short = 'H', long = "host", default_value = "127.0.0.1")]
    pub host: String,
}
