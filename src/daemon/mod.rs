pub mod protocol;

use crate::daemon::protocol::{Request, Response};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;
use crate::vault::store::{default_vault_path, load_or_init, save};
use rand::RngCore;
use secrecy::SecretString;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const RUNTIME_DIR: &str = ".envkey/run";
const SOCKET_FILE: &str = "envkeyd.sock";
const MAX_LIFETIME: Duration = Duration::from_secs(60 * 60);
const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

struct Session {
    password: SecretString,
    created_at: Instant,
    last_used_at: Instant,
    locked: bool,
}

pub fn run_daemon() -> AppResult<()> {
    let socket = socket_path()?;
    if let Some(parent) = socket.parent() {
        fs::create_dir_all(parent)?;
        set_dir_permissions(parent)?;
    }

    if socket.exists() {
        let _ = fs::remove_file(&socket);
    }

    let listener = UnixListener::bind(&socket)?;
    let mut sessions: HashMap<String, Session> = HashMap::new();

    loop {
        let (mut stream, _) = listener.accept()?;
        let req = read_message::<Request>(&mut stream)?;
        let resp = handle_request(req, &mut sessions);
        write_message(&mut stream, &resp)?;
    }
}

pub fn ensure_daemon_running() -> AppResult<()> {
    if health_check().is_ok() {
        return Ok(());
    }

    let exe = std::env::current_exe()?;
    Command::new(exe)
        .arg("__daemon_serve")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        if health_check().is_ok() {
            return Ok(());
        }
    }

    Err(AppError::DaemonUnavailable)
}

pub fn health_check() -> AppResult<()> {
    match request(Request::Health)? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn auth_unlock(password: String) -> AppResult<String> {
    ensure_daemon_running()?;
    match request(Request::AuthUnlock { password })? {
        Response::AuthUnlockOk { token } => Ok(token),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn auth_status(session: Option<String>) -> AppResult<(bool, bool, bool)> {
    ensure_daemon_running()?;
    match request(Request::AuthStatus { session })? {
        Response::AuthStatusOk {
            daemon_running,
            session_valid,
            locked,
        } => Ok((daemon_running, session_valid, locked)),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn auth_lock(session: String) -> AppResult<()> {
    match request(Request::AuthLock { session })? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn auth_logout(session: String) -> AppResult<()> {
    match request(Request::AuthLogout { session })? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_add(session: String, profile: &str, key: &str, value: &str) -> AppResult<()> {
    match request(Request::VaultAdd {
        session,
        profile: profile.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    })? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_run(
    session: String,
    profile: &str,
    args: &[String],
) -> AppResult<(i32, String, String)> {
    match request(Request::VaultRun {
        session,
        profile: profile.to_string(),
        args: args.to_vec(),
    })? {
        Response::VaultRunOk {
            code,
            stdout,
            stderr,
        } => Ok((code, stdout, stderr)),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_env(session: String, profile: &str) -> AppResult<Vec<String>> {
    match request(Request::VaultEnv {
        session,
        profile: profile.to_string(),
    })? {
        Response::VaultEnvOk { lines } => Ok(lines),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_profiles(session: String) -> AppResult<Vec<String>> {
    match request(Request::VaultProfiles { session })? {
        Response::VaultProfilesOk { profiles } => Ok(profiles),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_key_remove(session: String, profile: &str, key: &str) -> AppResult<()> {
    match request(Request::VaultKeyRemove {
        session,
        profile: profile.to_string(),
        key: key.to_string(),
    })? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub fn vault_profile_remove(session: String, profile: &str) -> AppResult<()> {
    match request(Request::VaultProfileRemove {
        session,
        profile: profile.to_string(),
    })? {
        Response::Ok => Ok(()),
        Response::Error { code, message } => Err(AppError::DaemonResponse { code, message }),
        other => Err(AppError::Protocol(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

fn request(req: Request) -> AppResult<Response> {
    let socket = socket_path()?;
    let mut stream = UnixStream::connect(socket).map_err(|_| AppError::DaemonUnavailable)?;
    write_message(&mut stream, &req)?;
    read_message(&mut stream)
}

fn read_message<T: for<'de> serde::Deserialize<'de>>(stream: &mut UnixStream) -> AppResult<T> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

fn write_message<T: serde::Serialize>(stream: &mut UnixStream, msg: &T) -> AppResult<()> {
    let body = serde_json::to_vec(msg)?;
    let len = (body.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&body)?;
    stream.flush()?;
    Ok(())
}

fn socket_path() -> AppResult<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    Ok(PathBuf::from(home).join(RUNTIME_DIR).join(SOCKET_FILE))
}

fn set_dir_permissions(path: &std::path::Path) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn handle_request(req: Request, sessions: &mut HashMap<String, Session>) -> Response {
    sweep_sessions(sessions);

    let result = match req {
        Request::Health => Ok(Response::Ok),
        Request::AuthUnlock { password } => do_auth_unlock(sessions, password),
        Request::AuthStatus { session } => do_auth_status(sessions, session),
        Request::AuthLock { session } => do_auth_lock(sessions, &session),
        Request::AuthLogout { session } => {
            sessions.remove(&session);
            Ok(Response::Ok)
        }
        Request::VaultAdd {
            session,
            profile,
            key,
            value,
        } => do_vault_add(sessions, &session, &profile, &key, &value),
        Request::VaultRun {
            session,
            profile,
            args,
        } => do_vault_run(sessions, &session, &profile, &args),
        Request::VaultEnv { session, profile } => do_vault_env(sessions, &session, &profile),
        Request::VaultProfiles { session } => do_vault_profiles(sessions, &session),
        Request::VaultKeyRemove {
            session,
            profile,
            key,
        } => do_vault_key_remove(sessions, &session, &profile, &key),
        Request::VaultProfileRemove { session, profile } => {
            do_vault_profile_remove(sessions, &session, &profile)
        }
    };

    match result {
        Ok(resp) => resp,
        Err((code, message)) => Response::Error { code, message },
    }
}

type ServerResult<T> = Result<T, (String, String)>;

fn do_auth_unlock(
    sessions: &mut HashMap<String, Session>,
    password: String,
) -> ServerResult<Response> {
    let pw = SecretString::new(password);
    let path = default_vault_path().map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))?;
    let _ = load_or_init(&path, &pw).map_err(|e| ("UNAUTHORIZED".to_string(), e.to_string()))?;

    let token = random_token();
    let now = Instant::now();
    sessions.insert(
        token.clone(),
        Session {
            password: pw,
            created_at: now,
            last_used_at: now,
            locked: false,
        },
    );

    Ok(Response::AuthUnlockOk { token })
}

fn do_auth_status(
    sessions: &mut HashMap<String, Session>,
    session: Option<String>,
) -> ServerResult<Response> {
    let (session_valid, locked) = if let Some(token) = session {
        if let Some(s) = sessions.get(&token) {
            (true, s.locked)
        } else {
            (false, false)
        }
    } else {
        (false, false)
    };

    Ok(Response::AuthStatusOk {
        daemon_running: true,
        session_valid,
        locked,
    })
}

fn do_auth_lock(sessions: &mut HashMap<String, Session>, session: &str) -> ServerResult<Response> {
    let s = sessions.get_mut(session).ok_or_else(|| {
        (
            "SESSION_MISSING".to_string(),
            "Session not found".to_string(),
        )
    })?;
    s.locked = true;
    Ok(Response::Ok)
}

fn do_vault_add(
    sessions: &mut HashMap<String, Session>,
    session: &str,
    profile: &str,
    key: &str,
    value: &str,
) -> ServerResult<Response> {
    let pw = validate_session(sessions, session)?;
    let mut vault = load_vault(&pw)?;
    vault
        .profiles
        .entry(profile.to_string())
        .or_default()
        .vars
        .insert(key.to_string(), value.to_string());
    persist_vault(&vault, &pw)?;
    Ok(Response::Ok)
}

fn do_vault_run(
    sessions: &mut HashMap<String, Session>,
    session: &str,
    profile: &str,
    args: &[String],
) -> ServerResult<Response> {
    if args.is_empty() {
        return Err((
            "BAD_REQUEST".to_string(),
            "No command provided to run".to_string(),
        ));
    }

    let pw = validate_session(sessions, session)?;
    let vault = load_vault(&pw)?;
    let selected = vault.profiles.get(profile).ok_or_else(|| {
        (
            "PROFILE_NOT_FOUND".to_string(),
            format!("Profile not found: {profile}"),
        )
    })?;

    let output = Command::new(&args[0])
        .args(&args[1..])
        .envs(&selected.vars)
        .output()
        .map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))?;

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(Response::VaultRunOk {
        code,
        stdout,
        stderr,
    })
}

fn do_vault_env(
    sessions: &mut HashMap<String, Session>,
    session: &str,
    profile: &str,
) -> ServerResult<Response> {
    let pw = validate_session(sessions, session)?;
    let vault = load_vault(&pw)?;
    let selected = vault.profiles.get(profile).ok_or_else(|| {
        (
            "PROFILE_NOT_FOUND".to_string(),
            format!("Profile not found: {profile}"),
        )
    })?;

    let mut vars: Vec<(&String, &String)> = selected.vars.iter().collect();
    vars.sort_by(|(a, _), (b, _)| a.cmp(b));
    let lines = vars
        .into_iter()
        .map(|(k, v)| format!("export {k}={}", shell_single_quote(v)))
        .collect();

    Ok(Response::VaultEnvOk { lines })
}

fn do_vault_profiles(
    sessions: &mut HashMap<String, Session>,
    session: &str,
) -> ServerResult<Response> {
    let pw = validate_session(sessions, session)?;
    let vault = load_vault(&pw)?;
    let mut profiles: Vec<String> = vault.profiles.keys().cloned().collect();
    profiles.sort();
    Ok(Response::VaultProfilesOk { profiles })
}

fn do_vault_key_remove(
    sessions: &mut HashMap<String, Session>,
    session: &str,
    profile: &str,
    key: &str,
) -> ServerResult<Response> {
    let pw = validate_session(sessions, session)?;
    let mut vault = load_vault(&pw)?;

    let selected = vault.profiles.get_mut(profile).ok_or_else(|| {
        (
            "PROFILE_NOT_FOUND".to_string(),
            format!("Profile not found: {profile}"),
        )
    })?;

    if !selected.vars.contains_key(key) {
        return Err((
            "KEY_NOT_FOUND".to_string(),
            format!("Key not found in profile '{profile}': {key}"),
        ));
    }

    selected.vars.remove(key);
    persist_vault(&vault, &pw)?;
    Ok(Response::Ok)
}

fn do_vault_profile_remove(
    sessions: &mut HashMap<String, Session>,
    session: &str,
    profile: &str,
) -> ServerResult<Response> {
    let pw = validate_session(sessions, session)?;
    let mut vault = load_vault(&pw)?;

    if !vault.profiles.contains_key(profile) {
        return Err((
            "PROFILE_NOT_FOUND".to_string(),
            format!("Profile not found: {profile}"),
        ));
    }

    vault.profiles.remove(profile);
    persist_vault(&vault, &pw)?;
    Ok(Response::Ok)
}

fn load_vault(password: &SecretString) -> ServerResult<Vault> {
    let path = default_vault_path().map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))?;
    load_or_init(&path, password).map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))
}

fn persist_vault(vault: &Vault, password: &SecretString) -> ServerResult<()> {
    let path = default_vault_path().map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))?;
    save(&path, vault, password).map_err(|e| ("VAULT_ERROR".to_string(), e.to_string()))
}

fn validate_session(
    sessions: &mut HashMap<String, Session>,
    token: &str,
) -> ServerResult<SecretString> {
    let session = sessions.get_mut(token).ok_or_else(|| {
        (
            "SESSION_MISSING".to_string(),
            "No active session".to_string(),
        )
    })?;

    if session.locked {
        return Err((
            "SESSION_LOCKED".to_string(),
            "Session is locked".to_string(),
        ));
    }

    let now = Instant::now();
    if now.duration_since(session.created_at) > MAX_LIFETIME
        || now.duration_since(session.last_used_at) > IDLE_TIMEOUT
    {
        return Err(("SESSION_EXPIRED".to_string(), "Session expired".to_string()));
    }

    session.last_used_at = now;
    Ok(session.password.clone())
}

fn sweep_sessions(sessions: &mut HashMap<String, Session>) {
    let now = Instant::now();
    sessions.retain(|_, s| {
        now.duration_since(s.created_at) <= MAX_LIFETIME
            && now.duration_since(s.last_used_at) <= IDLE_TIMEOUT
    });
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let mut out = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

fn shell_single_quote(input: &str) -> String {
    let escaped = input.replace('\'', "'\\''");
    format!("'{escaped}'")
}
