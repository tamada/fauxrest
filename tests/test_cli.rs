//! Tests that run the built binary, for behaviour that only exists at the
//! process boundary: which stream output goes to, and the exit status.

use std::process::{Command, Output};

/// Runs the binary with `args`.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fauxrest"))
        .args(args)
        .output()
        .expect("the binary should run")
}

/// `--help` and `--version` are requests that succeeded, so they belong on
/// stdout with status 0. `clap` reports both as errors, and treating every
/// `clap::Error` as a failure sent them to stderr with status 1 — enough to
/// make `fauxrest --help | less` show nothing.
#[test]
fn test_help_and_version_go_to_stdout_and_succeed() {
    for flag in ["--help", "-h", "--version", "-V"] {
        let out = run(&[flag]);
        assert!(
            out.status.success(),
            "{} should exit 0, got {:?}",
            flag,
            out.status.code()
        );
        assert!(!out.stdout.is_empty(), "{} should write to stdout", flag);
        assert!(
            out.stderr.is_empty(),
            "{} should leave stderr empty, got: {}",
            flag,
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// `--help` has to name the flags it documents, since the reference page is
/// generated from it.
#[test]
fn test_help_lists_the_flags() {
    let out = run(&["--help"]);
    let help = String::from_utf8_lossy(&out.stdout);
    for flag in [
        "--layout",
        "--dest",
        "--serializer",
        "--minify",
        "--overwrite",
    ] {
        assert!(help.contains(flag), "--help should mention {}", flag);
    }
    assert!(
        !help.contains("--format"),
        "--format does not exist and must not be advertised"
    );
}

/// A usage mistake is a failure, and the convention for one is status 2 on
/// stderr — distinct from a build that started and then failed.
#[test]
fn test_usage_error_reports_on_stderr_with_status_two() {
    let out = run(&["--no-such-flag"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        out.stdout.is_empty(),
        "usage errors do not belong on stdout"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--no-such-flag"),
        "the error should name the offending argument"
    );
}

/// A build that fails keeps its own status, which is not the parser's.
#[test]
fn test_build_failure_exits_one() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(&[
        tmp.path().join("no-such-data-dir").to_str().unwrap(),
        "--dest",
        tmp.path().join("dist").to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        !out.stderr.is_empty(),
        "a failed build should say why on stderr"
    );
}
