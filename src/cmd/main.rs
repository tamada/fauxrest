use clap::{Parser, ValueEnum};
use fauxrest::{Config, Error, Layout, Result};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "fauxrest", version, about, long_about = None)]
pub struct Args {
    /// Path to the input data directory.
    #[arg(
        help = "Path to the input data directory",
        value_name = "DATA_DIR",
        default_value = "data"
    )]
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
        help = "Path to the output directory [default: dist]",
        value_name = "DEST_DIR"
    )]
    dest: Option<PathBuf>,

    #[clap(
        short,
        long,
        help = "Serializer to use for the output. [available: json, typescript, sql] [default: json]",
        value_name = "SERIALIZER"
    )]
    serializer: Option<String>,

    #[clap(long, default_value_t = false, help = "If true, minify the output")]
    minify: bool,

    #[clap(
        long,
        default_value_t = false,
        help = "If true, overwrite existing files in the destination directory"
    )]
    overwrite: bool,

    #[cfg(debug_assertions)]
    #[clap(long, default_value_t = false, help = "Generate completion files")]
    gencomp: bool,
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
    /// Otherwise, it attempts to discover the config in the inputs directory,
    /// or falls back to the default configuration.
    ///
    /// Options that the user explicitly passes on the command line always take
    /// precedence over the loaded configuration file, which in turn takes
    /// precedence over the built-in defaults (CLI > config > default). When the
    /// loaded configuration provides serializers, every explicitly given CLI
    /// option (`--dest`, `--serializer`, `--layout`, `--minify`, `--overwrite`)
    /// overrides the matching field of each serializer entry.
    pub(crate) fn load_config(&self) -> Result<Config> {
        let config = if let Some(config) = &self.config {
            fauxrest::Config::load_from_file(config)
        } else if let Some(discovered_path) = Self::discover(Path::new(&self.inputs)) {
            fauxrest::Config::load_from_file(&discovered_path)
        } else {
            Ok(fauxrest::Config::default())
        };
        match config {
            Ok(mut config) => {
                if config.serializers.is_empty() {
                    config.serializers = vec![self.serializer_config()];
                } else {
                    self.apply_cli_overrides(&mut config);
                }
                Ok(config)
            }
            Err(e) => Err(e),
        }
    }

    /// Applies the options that were explicitly given on the command line onto
    /// every serializer entry of the loaded configuration. Options that were not
    /// given keep the value coming from the configuration file.
    fn apply_cli_overrides(&self, config: &mut Config) {
        for serializer in config.serializers.iter_mut() {
            if let Some(dest) = &self.dest {
                serializer.dest = dest.clone();
            }
            if let Some(s) = &self.serializer {
                serializer.serializer = s.clone();
            }
            if let Some(layout) = &self.layout {
                serializer.layout = layout.clone();
            }
            if self.minify {
                serializer.minify = true;
            }
            // When `--overwrite` is passed on the command line it forces
            // every serializer to allow overwriting, regardless of the
            // value present in the loaded configuration file.
            if self.overwrite {
                serializer.overwrite = true;
            }
        }
    }

    /// Builds a single serializer configuration from the command line options,
    /// falling back to the built-in defaults for any option that was not given.
    fn serializer_config(&self) -> fauxrest::SerializerConfig {
        fauxrest::SerializerConfig {
            layout: self.layout.clone().unwrap_or(Layout::Index),
            serializer: self
                .serializer
                .clone()
                .unwrap_or_else(|| String::from("json")),
            dest: self.dest.clone().unwrap_or_else(|| PathBuf::from("dist")),
            minify: self.minify,
            overwrite: self.overwrite,
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

mod gencomp;

fn perform_build(args: Args) -> Result<()> {
    #[cfg(debug_assertions)]
    if args.gencomp {
        gencomp::_generate("assets/completions");
        return Ok(());
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use fauxrest::Layout;
    use std::io::Write;

    /// Writes a config file with a single `$config` serializer entry into a
    /// fresh temp directory and returns the directory handle and the config path.
    fn write_config(body: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        (dir, path)
    }

    const CONFIG_BODY: &str =
        r#"{"$config": [{"serializer": "typescript", "layout": "file", "dest": "from-config", "minify": true}]}"#;

    #[test]
    fn config_values_used_when_no_cli_override() {
        let (_dir, path) = write_config(CONFIG_BODY);
        let args = Args::try_parse_from(["fauxrest", "data", "-c", path.to_str().unwrap()]).unwrap();
        let config = args.load_config().unwrap();
        assert_eq!(config.serializers.len(), 1);
        let s = &config.serializers[0];
        assert_eq!(s.serializer, "typescript");
        assert_eq!(s.dest, PathBuf::from("from-config"));
        assert!(matches!(s.layout, Layout::File));
        assert!(s.minify);
    }

    #[test]
    fn explicit_dest_overrides_config() {
        let (_dir, path) = write_config(CONFIG_BODY);
        let args = Args::try_parse_from([
            "fauxrest",
            "data",
            "-c",
            path.to_str().unwrap(),
            "-d",
            "cli-dest",
        ])
        .unwrap();
        let config = args.load_config().unwrap();
        let s = &config.serializers[0];
        // dest comes from the CLI, the rest is still from the config.
        assert_eq!(s.dest, PathBuf::from("cli-dest"));
        assert_eq!(s.serializer, "typescript");
        assert!(matches!(s.layout, Layout::File));
        assert!(s.minify);
    }

    #[test]
    fn explicit_serializer_and_layout_override_config() {
        let (_dir, path) = write_config(CONFIG_BODY);
        let args = Args::try_parse_from([
            "fauxrest",
            "data",
            "-c",
            path.to_str().unwrap(),
            "-s",
            "json",
            "-l",
            "index",
        ])
        .unwrap();
        let config = args.load_config().unwrap();
        let s = &config.serializers[0];
        assert_eq!(s.serializer, "json");
        assert!(matches!(s.layout, Layout::Index));
        // dest is untouched by the CLI here.
        assert_eq!(s.dest, PathBuf::from("from-config"));
    }

    #[test]
    fn minify_flag_overrides_config() {
        // config has minify=false, CLI passes --minify.
        let (_dir, path) = write_config(
            r#"{"$config": [{"serializer": "json", "layout": "index", "dest": "from-config"}]}"#,
        );
        let args = Args::try_parse_from([
            "fauxrest",
            "data",
            "-c",
            path.to_str().unwrap(),
            "--minify",
        ])
        .unwrap();
        let config = args.load_config().unwrap();
        assert!(config.serializers[0].minify);
    }

    #[test]
    fn overrides_apply_to_every_serializer_entry() {
        let (_dir, path) = write_config(
            r#"{"$config": [
                {"serializer": "json", "layout": "index", "dest": "a"},
                {"serializer": "sqlite", "layout": "file", "dest": "b"}
            ]}"#,
        );
        let args = Args::try_parse_from([
            "fauxrest",
            "data",
            "-c",
            path.to_str().unwrap(),
            "-d",
            "cli-dest",
        ])
        .unwrap();
        let config = args.load_config().unwrap();
        assert_eq!(config.serializers.len(), 2);
        for s in &config.serializers {
            assert_eq!(s.dest, PathBuf::from("cli-dest"));
        }
    }

    #[test]
    fn no_config_uses_defaults_without_cli_options() {
        // "no-such-dir" has no discoverable config, so the default config is used.
        let args = Args::try_parse_from(["fauxrest", "no-such-dir"]).unwrap();
        let config = args.load_config().unwrap();
        let s = &config.serializers[0];
        assert_eq!(s.serializer, "json");
        assert_eq!(s.dest, PathBuf::from("dist"));
        assert!(matches!(s.layout, Layout::Index));
        assert!(!s.minify);
    }

    #[test]
    fn no_config_honors_explicit_cli_options() {
        // Even without a config file, an explicit --dest must be honored.
        let args =
            Args::try_parse_from(["fauxrest", "no-such-dir", "-d", "cli-dest", "-s", "sqlite"])
                .unwrap();
        let config = args.load_config().unwrap();
        let s = &config.serializers[0];
        assert_eq!(s.dest, PathBuf::from("cli-dest"));
        assert_eq!(s.serializer, "sqlite");
    }
}
