use crate::commands::session::require_session;
use crate::daemon;
use crate::error::{AppError, AppResult};
use std::io::{self, Write};

pub fn execute(profile: &str, yes: bool) -> AppResult<()> {
    if !yes && !confirm(&format!("Delete profile '{}' and all its keys?", profile))? {
        return Err(AppError::OperationCancelled);
    }

    let session = require_session()?;
    daemon::vault_profile_remove(session, profile)?;
    println!("Removed profile '{}'", profile);
    Ok(())
}

fn confirm(prompt: &str) -> AppResult<bool> {
    eprint!("{} [y/N]: ", prompt);
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
