use clap::{Parser, Subcommand};
use fauxrest::{Config, Layout, Result};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "fauxrest", version, about = "Static API Generator", long_about = None)]
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

    #[clap(long, default_value_t = false)]
    minify: bool,
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
    /// Loads the configuration based on the command line options.
    /// If an explicit config path is provided, it attempts to load it.
    /// If an explicit layout is provided, it creates a new Config.
    /// Otherwise, it attempts to discover the config in the inputs directory,
    /// or falls back to the default configuration.
    pub(crate) fn load_config(&self) -> Result<Config> {
        let config = if let Some(config) = &self.config {
            fauxrest::Config::load_from_file(config)
        } else if let Some(discovered_path) = Self::discover(Path::new(&self.inputs)) {
            fauxrest::Config::load_from_file(&discovered_path)
        } else {
            Ok(fauxrest::Config::default())
        };
        match config {
            Ok(config) => {
                if config.serializers.is_empty() {
                    Ok(fauxrest::Config {
                        serializers: vec![self.serializer_config()],
                        api: config.api,
                    })
                } else {
                    Ok(config)
                }
            }
            Err(e) => Err(e),
        }
    }

    fn serializer_config(&self) -> fauxrest::SerializerConfig {
        let layout = if let Some(l) = &self.layout {
            l.clone()
        } else {
            Layout::Index
        };
        fauxrest::SerializerConfig {
            layout,
            serializer: self.serializer.clone(),
            dest: self.dest.clone(),
            minify: self.minify,
        }
    }

    /// Discovers and loads a configuration file from a directory.
    /// It searches for '_config.json', '_fauxrest.json', '.config.json', and '.fauxrest.json' in order.
    fn discover(dir: &Path) -> Option<PathBuf> {
        let configs = [
            "_config.json",
            "_fauxrest.json",
            ".config.json",
            ".fauxrest.json",
        ];
        configs
            .iter()
            .map(|c| dir.join(c))
            .find(|path| path.exists())
    }
}

fn perform_build(args: Args) -> Result<()> {
    let config = args.load_config()?;
    fauxrest::run(config, PathBuf::from(args.inputs))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Build(args)) => perform_build(args),
        Some(Commands::Serve { args, port }) => {
            // Note: The original 'serve' functionality needs to be implemented.
            // For now, it's a placeholder as requested by the architecture.
            unimplemented!(
                "Starting server on port {} with inputs {} and config {:?}",
                port,
                args.inputs,
                args.config
            );
        }
        None => {
            if let Some(args) = cli.args {
                perform_build(args)
            } else {
                Err(fauxrest::Error::Config("missing inputs".into()))
            }
        }
    }
}
