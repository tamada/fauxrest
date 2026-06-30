use clap::{Parser, ValueEnum};
use fauxrest::{Config, Error, Layout, Result};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "fauxrest", version, about, long_about = None)]
pub struct Args {
    /// Path to the input data directory.
    #[arg(help = "Path to the input data directory", value_name = "DATA_DIR")]
    inputs: String,

    #[clap(short = 'L', long, help = "Specify the log level", value_enum, default_value_t = LogLevel::Warn, value_name = "LEVEL")]
    level: LogLevel,

    #[clap(
        short,
        long,
        help = "Path to the configuration file",
        value_name = "CONFIG_FILE"
    )]
    config: Option<PathBuf>,

    #[clap(
        short,
        long,
        help = "Layout to use for the output",
        value_name = "LAYOUT"
    )]
    layout: Option<Layout>,

    #[clap(
        short,
        long,
        default_value = "dist",
        help = "Path to the output directory",
        value_name = "DEST_DIR"
    )]
    dest: PathBuf,

    #[clap(short, long, help = "Serializer to use for the output. [available: json, typescript, sql]", default_value_t = String::from("json"), value_name = "SERIALIZER")]
    serializer: String,

    #[clap(long, default_value_t = false, help = "If true, minify the output")]
    minify: bool,
}

#[derive(Parser, ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
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
    let r = match Args::try_parse() {
        Ok(args) => perform_build(args),
        Err(e) => Err(Error::Clap(e)),
    };
    if let Err(e) = r {
        eprint!("{}", e);
        std::process::exit(1);
    }
    Ok(())
}
