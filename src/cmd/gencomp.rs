//! Shell completion file generation for the `fauxrest` CLI (debug builds
//! only, gated behind the `--gencomp` flag; see `src/cmd/main.rs`).

use std::path::Path;

#[cfg(debug_assertions)]
mod completions {
    use clap::{Command, CommandFactory};
    use clap_complete::Shell;
    use std::fs::File;
    use std::path::Path;

    /// Generates a single shell's completion script for `app` and writes it
    /// to `outdir/{shell}/{filename}`, creating the shell subdirectory if
    /// needed.
    fn perform(s: Shell, app: &mut Command, appname: &str, outdir: &Path, filename: String) {
        let destfile = outdir.join(format!("{s}")).join(filename);
        std::fs::create_dir_all(destfile.parent().unwrap()).unwrap();
        if let Ok(mut dest) = File::create(destfile) {
            clap_complete::generate(s, app, appname, &mut dest);
        }
    }

    /// Generates completion scripts for all supported shells (bash, elvish,
    /// fish, PowerShell, zsh) from the [`crate::Args`] clap definition,
    /// writing them under `outdir`.
    pub(super) fn generate(outdir: &Path) {
        use Shell::{Bash, Elvish, Fish, PowerShell, Zsh};
        let name = "fauxrest";

        let mut app = crate::Args::command();
        app.set_bin_name(name);

        perform(Bash, &mut app, name, outdir, name.into());
        perform(Elvish, &mut app, name, outdir, format!("{name}.elv"));
        perform(Fish, &mut app, name, outdir, format!("{name}.fish"));
        perform(PowerShell, &mut app, name, outdir, format!("{name}.ps1"));
        perform(Zsh, &mut app, name, outdir, format!("_{name}"));
    }
}

/// Generates shell completion files into `_outdir` (debug builds only).
///
/// In release builds this is a no-op, so the `--gencomp` flag (which is
/// itself only compiled in debug builds) has no runtime cost.
pub(crate) fn _generate<P: AsRef<Path>>(_outdir: P) {
    #[cfg(debug_assertions)]
    completions::generate(_outdir.as_ref());
}
