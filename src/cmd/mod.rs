pub mod args;

use args::{Cli, Commands, ServeArgs};
use clap::Parser;
use prest::Generator;

/// Main entrypoint for executing the CLI logic.
///
/// This parses options and dispatches to subcommands or default build action.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Build(args)) => execute_build(&args.inputs, &args.dest),
        Some(Commands::Serve(args)) => execute_serve(&args),
        None => execute_build(&cli.inputs, &cli.dest),
    }
}

/// Executes the static api build operation.
fn execute_build(inputs: &str, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
    let generator = Generator::new(inputs, dest);
    generator.generate()?;
    println!("API successfully compiled to: {}", dest);
    Ok(())
}

/// Starts the lightweight development/serve server.
fn execute_serve(args: &ServeArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting server on http://{}:{}", args.host, args.port);
    println!("Serving data from: {}", args.inputs);
    let generator = Generator::new(&args.inputs, "./dist");
    generator.generate()?;
    Ok(())
}
