use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "envkey")]
#[command(about = "Run commands with securely injected environment variables")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Add or update a secret in a profile
    Add {
        #[arg(short, long)]
        profile: String,
        key: String,
        value: Option<String>,
    },
    /// Run a command with variables from a profile
    Run {
        #[arg(short, long)]
        profile: String,
        #[arg(last = true, required = true)]
        args: Vec<String>,
    },
    /// Print shell export lines for variables in a profile
    Env {
        #[arg(short, long)]
        profile: String,
    },
    /// List available profile names
    Profiles,
    /// Remove a profile and all its keys
    ProfileRm {
        #[arg(short, long)]
        profile: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Remove a single key from a profile
    KeyRm {
        #[arg(short, long)]
        profile: String,
        key: String,
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
