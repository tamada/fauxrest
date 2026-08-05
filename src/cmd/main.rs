//! `fauxrest` command-line entry point.
//!
//! Parses CLI arguments into [`Args`], resolves the effective [`Config`]
//! (explicit `--config`, a discovered `_config.json`, CLI flags, or the
//! built-in default), and runs the build via [`fauxrest::run`].

use clap::{Parser, ValueEnum};
use fauxrest::{Config, Layout, Result};
use std::path::{Path, PathBuf};

/// Command-line arguments for the `fauxrest` binary.
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

    /// Specify the log level.
    #[clap(short = 'L', long, help = "Specify the log level", value_enum, default_value_t = LogLevel::Warn, value_name = "LEVEL")]
    level: LogLevel,

    /// Path to the configuration file. When unset, [`Args::load_config`]
    /// falls back to discovering one in the input data directory.
    #[clap(
        short,
        long,
        help = "Path to the configuration file",
        value_name = "CONFIG_FILE"
    )]
    config: Option<PathBuf>,

    /// Delivery layout to use for the output, when building the
    /// configuration from CLI flags instead of a config file.
    #[clap(
        short,
        long,
        help = "Layout to use for the output",
        value_name = "LAYOUT"
    )]
    layout: Option<Layout>,

    /// Path to the output directory.
    #[clap(
        short,
        long,
        help = "Path to the output directory [default: dist]",
        value_name = "DEST_DIR"
    )]
    dest: Option<PathBuf>,

    /// Serializer to use for the output (`json`, `typescript`, or `sql`).
    /// Defaults to `json` when neither this flag nor a config file sets one.
    #[clap(
        short,
        long,
        help = "Serializer to use for the output. [available: json, typescript, sql] [default: json]",
        value_name = "SERIALIZER"
    )]
    serializer: Option<String>,

    /// If `true`, write the whole API as one `api.[ext]` instead of a file
    /// per endpoint.
    #[clap(
        long,
        default_value_t = false,
        help = "If set, write the whole API as one api.[ext] file"
    )]
    bundle: bool,

    /// If `true`, minify the serialized output.
    #[clap(long, default_value_t = false, help = "If true, minify the output")]
    minify: bool,

    /// If set, disable minification, overriding any `minify: true` in the
    /// loaded configuration. Conflicts with `--minify`.
    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "minify",
        help = "If set, disable minification (overrides config)"
    )]
    no_minify: bool,

    /// If `true`, allow writing into a non-empty destination directory
    /// (overwriting existing files).
    #[clap(
        long,
        default_value_t = false,
        help = "If true, overwrite existing files in the destination directory"
    )]
    overwrite: bool,

    /// If `true`, copy all non-JSON static files from the data directory
    /// into each destination (allow all); `$static` exclude globs still
    /// take precedence.
    #[clap(
        long,
        default_value_t = false,
        help = "Copy all non-JSON static files from the data directory into each destination (allow all). $static exclude globs still take precedence."
    )]
    copy_static: bool,

    /// If `true` (debug builds only), generate shell completion files
    /// instead of running the build.
    #[cfg(debug_assertions)]
    #[clap(long, default_value_t = false, help = "Generate completion files")]
    gencomp: bool,
}

/// Verbosity level for CLI logging output.
#[derive(Parser, ValueEnum, Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    /// Only report errors.
    Error,
    /// Report warnings and errors (the default).
    Warn,
    /// Report informational messages, warnings, and errors.
    Info,
    /// Report debug-level details in addition to `Info`.
    Debug,
    /// Report the most verbose, low-level tracing output.
    Trace,
}

impl Args {
    /// Loads the configuration based on the command line options.
    ///
    /// Resolution order:
    /// 1. If `--config` is set, load that file.
    /// 2. Otherwise, try to [`discover`](Self::discover) a config file in
    ///    the input data directory.
    /// 3. Otherwise, fall back to [`Config::default`].
    ///
    /// Options that the user explicitly passes on the command line always take
    /// precedence over the loaded configuration file, which in turn takes
    /// precedence over the built-in defaults (CLI > config > default). When the
    /// loaded configuration provides serializers, every explicitly given CLI
    /// option (`--dest`, `--serializer`, `--layout`, `--minify`, `--no-minify`,
    /// `--overwrite`) overrides the matching field of each serializer entry
    /// (see [`Args::apply_cli_overrides`]); when it provides none, a single
    /// entry is synthesized from the CLI flags (see
    /// [`Args::serializer_config`]), keeping the loaded routing overlay (if
    /// any). `--copy-static` additionally enables copying of all non-JSON
    /// static files.
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
                // `--copy-static` forces every non-JSON static file to be
                // treated as allowed. Any `$static` exclude globs from the
                // configuration file still take precedence (deny wins).
                if self.copy_static {
                    config.copy_static_all = true;
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
            if self.bundle {
                serializer.bundle = true;
            }
            if self.minify {
                serializer.minify = true;
            } else if self.no_minify {
                serializer.minify = false;
            }
            // When `--overwrite` is passed on the command line it forces
            // every serializer to allow overwriting, regardless of the
            // value present in the loaded configuration file.
            if self.overwrite {
                serializer.overwrite = true;
            }
        }
    }

    /// Builds a single [`fauxrest::SerializerConfig`] from the command line
    /// options
    /// (`--serializer`/`--layout`/`--dest`/`--bundle`/`--minify`/`--overwrite`),
    /// falling back to the built-in defaults (`json`, [`Layout::Index`],
    /// `dist`) for any option that was not given.
    fn serializer_config(&self) -> fauxrest::SerializerConfig {
        fauxrest::SerializerConfig {
            layout: self.layout.clone().unwrap_or_default(),
            serializer: self
                .serializer
                .clone()
                .unwrap_or_else(|| String::from("json")),
            dest: self.dest.clone().unwrap_or_else(|| PathBuf::from("dist")),
            bundle: self.bundle,
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

/// Shell completion file generation (debug builds only); see
/// [`gencomp::_generate`].
mod gencomp;

/// Runs the requested action for parsed CLI `args`: in debug builds, generates
/// shell completion files if `--gencomp` was passed; otherwise resolves the
/// configuration and runs the static API build via [`fauxrest::run`].
fn perform_build(args: Args) -> Result<()> {
    #[cfg(debug_assertions)]
    if args.gencomp {
        gencomp::_generate("assets/completions");
        return Ok(());
    }

    let config = args.load_config()?;
    fauxrest::run(config, PathBuf::from(args.inputs))
}

/// Binary entry point: parses CLI arguments, runs [`perform_build`], and on
/// error prints the error to stderr and exits the process with status 1.
fn main() -> Result<()> {
    let args = match Args::try_parse() {
        Ok(args) => args,
        // `clap` reports `--help` and `--version` as errors as well as genuine
        // usage mistakes. Treating them all as failures sent help and version
        // to stderr with status 1, so `fauxrest --help | less` showed nothing.
        // `exit` tells them apart: help and version to stdout with status 0,
        // usage errors to stderr with status 2.
        Err(e) => e.exit(),
    };
    if let Err(e) = perform_build(args) {
        eprintln!("{}", e);
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

    const CONFIG_BODY: &str = r#"{"$config": [{"serializer": "typescript", "layout": "file", "dest": "from-config", "minify": true}]}"#;

    /// Without CLI overrides, every serializer field comes from the config file.
    #[test]
    fn config_values_used_when_no_cli_override() {
        let (_dir, path) = write_config(CONFIG_BODY);
        let args =
            Args::try_parse_from(["fauxrest", "data", "-c", path.to_str().unwrap()]).unwrap();
        let config = args.load_config().unwrap();
        assert_eq!(config.serializers.len(), 1);
        let s = &config.serializers[0];
        assert_eq!(s.serializer, "typescript");
        assert_eq!(s.dest, PathBuf::from("from-config"));
        assert!(matches!(s.layout, Layout::File));
        assert!(s.minify);
    }

    /// An explicit `--dest` overrides the config file; other fields are kept.
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

    /// Explicit `--serializer`/`--layout` override the config file; `dest` is kept.
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

    /// `--minify` turns minification on even when the config file leaves it off.
    #[test]
    fn minify_flag_overrides_config() {
        // config has minify=false, CLI passes --minify.
        let (_dir, path) = write_config(
            r#"{"$config": [{"serializer": "json", "layout": "index", "dest": "from-config"}]}"#,
        );
        let args =
            Args::try_parse_from(["fauxrest", "data", "-c", path.to_str().unwrap(), "--minify"])
                .unwrap();
        let config = args.load_config().unwrap();
        assert!(config.serializers[0].minify);
    }

    /// `--no-minify` turns minification off even when the config file enables it.
    #[test]
    fn no_minify_flag_overrides_config() {
        // config has minify=true, CLI passes --no-minify.
        let (_dir, path) = write_config(CONFIG_BODY);
        let args = Args::try_parse_from([
            "fauxrest",
            "data",
            "-c",
            path.to_str().unwrap(),
            "--no-minify",
        ])
        .unwrap();
        let config = args.load_config().unwrap();
        assert!(!config.serializers[0].minify);
    }

    /// `--minify` and `--no-minify` are mutually exclusive at parse time.
    #[test]
    fn minify_and_no_minify_conflict() {
        let result = Args::try_parse_from(["fauxrest", "data", "--minify", "--no-minify"]);
        assert!(result.is_err());
    }

    /// Without a config file, `--no-minify` keeps the default (unminified).
    #[test]
    fn no_config_with_no_minify_stays_unminified() {
        // Without a config file, --no-minify keeps the default (false).
        let args = Args::try_parse_from(["fauxrest", "no-such-dir", "--no-minify"]).unwrap();
        let config = args.load_config().unwrap();
        assert!(!config.serializers[0].minify);
    }

    /// CLI overrides are applied to every `$config` serializer entry, not just the first.
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

    /// Without a config file or CLI options, the built-in defaults are used.
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

    /// Without a config file, explicitly given CLI options are still honored.
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
