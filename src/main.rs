mod cli;
mod commands;
mod error;
mod vault;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> error::AppResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add {
            profile,
            key,
            value,
        } => {
            commands::add::execute(&profile, &key, value.as_deref())?;
            Ok(())
        }
        Commands::Run { profile, args } => {
            let code = commands::run::execute(&profile, &args)?;
            std::process::exit(code);
        }
        Commands::Env { profile } => {
            commands::env::execute(&profile)?;
            Ok(())
        }
        Commands::Profiles => {
            commands::profiles::execute()?;
            Ok(())
        }
        Commands::ProfileRm { profile, yes } => {
            commands::profile_rm::execute(&profile, yes)?;
            Ok(())
        }
        Commands::KeyRm { profile, key, yes } => {
            commands::key_rm::execute(&profile, &key, yes)?;
            Ok(())
        }
    }
}
