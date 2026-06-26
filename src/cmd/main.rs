use clap::{Parser, Subcommand};
use prest::{Config, Result, Layout};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "prest", version, about = "Static API Generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[clap(flatten)]
    args: Option<Args>,
}

#[derive(Parser, Debug)]
pub struct Args {
    /// Path to the input data directory.
    inputs: String,

    /// Path to the configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[clap(short, long)]
    layout: Option<Layout>,

    #[clap(short, long, default_value = "dist")]
    dest: PathBuf,

    #[clap(short, long, default_value_t = String::from("json"))] 
    serializer: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Compiles source JSON data into structured static JSON endpoints.
    Build(Args),
    /// Starts a lightweight local development server.
    Serve {
        #[clap(flatten)]
        args: Args,

        /// Port to listen on.
        #[arg(short = 'p', long = "port", default_value = "8080")]
        port: u16,
    },
}

impl Args {
    pub(crate) fn load_config(&self) -> Result<Config> {
        Config::load_or_default(
            self.config.as_ref(),
            Path::new(&self.inputs),
            self.layout.as_ref(),
            self.serializer.clone(),
            self.dest.clone(),
        )
    }
}

fn perform_build(args: Args) -> Result<()> {
    let config = args.load_config()?;
    prest::run(config, PathBuf::from(args.inputs))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Build(args)) => perform_build(args),
        Some(Commands::Serve { args, port }) => {
            // Note: The original 'serve' functionality needs to be implemented.
            // For now, it's a placeholder as requested by the architecture.
            unimplemented!("Starting server on port {} with inputs {} and config {:?}", port, args.inputs, args.config);
        },
        None => {
            if let Some(args) = cli.args {
                perform_build(args)
            } else {
                Err(prest::Error::Config("missing inputs".into()))
            }

        }
    }
}
