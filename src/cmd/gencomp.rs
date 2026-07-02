use std::path::Path;

#[cfg(debug_assertions)]
mod completions {
    use clap::{Command, CommandFactory};
    use clap_complete::Shell;
    use std::fs::File;
    use std::path::Path;

    fn perform(s: Shell, app: &mut Command, appname: &str, outdir: &Path, filename: String) {
        let destfile = outdir.join(format!("{s}")).join(filename);
        std::fs::create_dir_all(destfile.parent().unwrap()).unwrap();
        if let Ok(mut dest) = File::create(destfile) {
            clap_complete::generate(s, app, appname, &mut dest);
        }
    }

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

pub(crate) fn _generate<P: AsRef<Path>>(_outdir: P) {
    #[cfg(debug_assertions)]
    completions::generate(_outdir.as_ref());
}
