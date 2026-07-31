//! Static-file copy module
//!
//! Copies non-JSON static files (images, CSS, fonts, …) found in the input
//! data directory into every serializer destination, preserving the
//! sub-directory structure.
//!
//! # Policy and priority
//!
//! Copying is **deny by default**. A file is copied only when it is allowed
//! and not denied:
//!
//! 1. Data (`.json`) files and configuration files (`_config.json`,
//!    `_fauxrest.json`, `.config.json`, `.fauxrest.json`) are **always
//!    excluded** — they are inputs, never static assets.
//! 2. A file matching any `exclude` (deny) glob is **never** copied. `exclude`
//!    always wins, even when `--copy-static` forces allow-all.
//! 3. Otherwise the file is copied when `--copy-static` is set (allow all) or
//!    when it matches an `include` (allow) glob.

use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs;
use std::path::Path;

use crate::config::StaticSpec;
use crate::{Config, Error, Result};

/// Configuration file names that are always excluded from static copying.
const CONFIG_FILE_NAMES: [&str; 4] = [
    "_config.json",
    "_fauxrest.json",
    ".config.json",
    ".fauxrest.json",
];

/// A resolved static-copy policy combining the configured include/exclude
/// globs with the command line allow-all flag.
struct StaticPolicy {
    /// Compiled include (allow) glob set.
    include: GlobSet,
    /// Whether any include glob was configured (an empty [`GlobSet`] cannot
    /// distinguish "no patterns" from "no match").
    has_include: bool,
    /// Compiled exclude (deny) glob set; matches here always win.
    exclude: GlobSet,
    /// Treat every file as allowed (set by `--copy-static`); excludes still
    /// apply.
    allow_all: bool,
}

impl StaticPolicy {
    /// Builds the policy from configuration, compiling the glob sets.
    fn build(spec: Option<&StaticSpec>, allow_all: bool) -> Result<Self> {
        let (includes, excludes): (&[String], &[String]) = match spec {
            Some(s) => (s.include(), s.exclude()),
            None => (&[], &[]),
        };
        Ok(Self {
            include: build_glob_set(includes)?,
            has_include: !includes.is_empty(),
            exclude: build_glob_set(excludes)?,
            allow_all,
        })
    }

    /// Returns `true` when the copy feature is inert (nothing could ever be
    /// copied), allowing the caller to skip the directory walk entirely.
    fn is_noop(&self) -> bool {
        !self.allow_all && !self.has_include
    }

    /// Decides whether the file at the given data-relative path (using `/`
    /// separators) should be copied.
    ///
    /// The `exclude` (deny) globs always win, so they are checked before any
    /// allow decision.
    fn should_copy(&self, rel: &str) -> bool {
        // Deny always wins.
        if self.exclude.is_match(rel) {
            return false;
        }
        if self.allow_all {
            return true;
        }
        self.has_include && self.include.is_match(rel)
    }
}

/// Compiles a slice of glob patterns into a [`GlobSet`], reporting invalid
/// patterns as [`Error::Config`].
fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| {
            Error::Config(format!("invalid $static glob pattern '{}': {}", pattern, e))
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| Error::Config(format!("failed to build $static glob set: {}", e)))
}

/// Copies allowed static files from `data_dir` into every serializer
/// destination, preserving sub-directory structure.
pub(crate) fn copy_static_files(config: &Config, data_dir: &Path) -> Result<()> {
    let policy = StaticPolicy::build(config.static_files.as_ref(), config.copy_static_all)?;
    if policy.is_noop() {
        return Ok(());
    }

    let dests: Vec<&Path> = config
        .serializers
        .iter()
        .map(|s| s.dest.as_path())
        .collect();
    if dests.is_empty() {
        return Ok(());
    }

    walk_and_copy(data_dir, data_dir, &policy, &dests)
}

/// Recursively walks `dir`, copying matching files into each destination.
fn walk_and_copy(root: &Path, dir: &Path, policy: &StaticPolicy, dests: &[&Path]) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(Error::Io)?;
    for entry in entries {
        let entry = entry.map_err(Error::Io)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(Error::Io)?;
        if file_type.is_dir() {
            walk_and_copy(root, &path, policy, dests)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if is_always_excluded(&file_name) {
            continue;
        }

        let Some(rel) = relative_slash_path(root, &path) else {
            continue;
        };
        if !policy.should_copy(&rel) {
            continue;
        }

        for dest in dests {
            let target = dest.join(&rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(Error::Io)?;
            }
            fs::copy(&path, &target).map_err(Error::Io)?;
        }
    }
    Ok(())
}

/// Returns `true` for files that must never be copied: JSON data files and the
/// well-known configuration file names.
fn is_always_excluded(file_name: &str) -> bool {
    if CONFIG_FILE_NAMES.contains(&file_name) {
        return true;
    }
    Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

/// Computes the path of `path` relative to `root`, joined with `/` separators
/// for stable, platform-independent glob matching.
fn relative_slash_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StaticConfig;

    /// Builds a shorthand [`StaticSpec::Include`] from string literals.
    fn spec_include(patterns: &[&str]) -> StaticSpec {
        StaticSpec::Include(patterns.iter().map(|s| s.to_string()).collect())
    }

    /// Builds a full [`StaticSpec::Detailed`] from include/exclude string
    /// literals.
    fn spec_detailed(include: &[&str], exclude: &[&str]) -> StaticSpec {
        StaticSpec::Detailed(StaticConfig {
            include: include.iter().map(|s| s.to_string()).collect(),
            exclude: exclude.iter().map(|s| s.to_string()).collect(),
        })
    }

    /// Without any configuration or `--copy-static`, the policy is a no-op
    /// and copies nothing.
    #[test]
    fn test_default_policy_copies_nothing() {
        let policy = StaticPolicy::build(None, false).unwrap();
        assert!(policy.is_noop());
        assert!(!policy.should_copy("logo.png"));
    }

    /// Include globs allow matching files and leave others uncopied.
    #[test]
    fn test_include_glob_allows_matching_files() {
        let spec = spec_include(&["*.png", "css/**"]);
        let policy = StaticPolicy::build(Some(&spec), false).unwrap();
        assert!(!policy.is_noop());
        assert!(policy.should_copy("logo.png"));
        assert!(policy.should_copy("css/site.css"));
        assert!(!policy.should_copy("notes.txt"));
    }

    /// An exclude glob denies a file even when `--copy-static` allows all.
    #[test]
    fn test_exclude_wins_over_allow_all() {
        let spec = spec_detailed(&[], &["secret/**"]);
        let policy = StaticPolicy::build(Some(&spec), true).unwrap();
        assert!(policy.should_copy("logo.png"));
        assert!(!policy.should_copy("secret/key.pem"));
    }

    /// An exclude glob denies a file even when an include glob matches it.
    #[test]
    fn test_exclude_wins_over_include() {
        let spec = spec_detailed(&["**/*.png"], &["private/**"]);
        let policy = StaticPolicy::build(Some(&spec), false).unwrap();
        assert!(policy.should_copy("img/logo.png"));
        assert!(!policy.should_copy("private/logo.png"));
    }

    /// JSON data files and well-known config file names are never copied.
    #[test]
    fn test_always_excluded_files() {
        assert!(is_always_excluded("users.json"));
        assert!(is_always_excluded("_config.json"));
        assert!(is_always_excluded(".fauxrest.json"));
        assert!(!is_always_excluded("logo.png"));
        assert!(!is_always_excluded("style.css"));
    }

    /// An invalid glob pattern is reported as a configuration error.
    #[test]
    fn test_invalid_glob_reports_config_error() {
        let spec = spec_include(&["a[b"]);
        let err = match StaticPolicy::build(Some(&spec), false) {
            Ok(_) => panic!("invalid glob should be rejected"),
            Err(e) => e,
        };
        assert!(matches!(err, Error::Config(_)));
        assert!(format!("{}", err).contains("invalid $static glob"));
    }
}
