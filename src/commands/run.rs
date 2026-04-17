use crate::commands::session::require_session;
use crate::daemon;
use crate::error::{AppError, AppResult};

pub fn execute(profile: &str, args: &[String]) -> AppResult<i32> {
    if args.is_empty() {
        return Err(AppError::MissingCommand);
    }

    let session = require_session()?;
    let (code, stdout, stderr) = daemon::vault_run(session, profile, args)?;

    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }

    Ok(code)
}
