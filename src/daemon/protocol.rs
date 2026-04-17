use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Health,
    AuthUnlock {
        password: String,
    },
    AuthStatus {
        session: Option<String>,
    },
    AuthLock {
        session: String,
    },
    AuthLogout {
        session: String,
    },
    VaultAdd {
        session: String,
        profile: String,
        key: String,
        value: String,
    },
    VaultRun {
        session: String,
        profile: String,
        args: Vec<String>,
    },
    VaultEnv {
        session: String,
        profile: String,
    },
    VaultProfiles {
        session: String,
    },
    VaultKeyRemove {
        session: String,
        profile: String,
        key: String,
    },
    VaultProfileRemove {
        session: String,
        profile: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    Ok,
    AuthUnlockOk {
        token: String,
    },
    AuthStatusOk {
        daemon_running: bool,
        session_valid: bool,
        locked: bool,
    },
    VaultEnvOk {
        lines: Vec<String>,
    },
    VaultProfilesOk {
        profiles: Vec<String>,
    },
    VaultRunOk {
        code: i32,
        stdout: String,
        stderr: String,
    },
    Error {
        code: String,
        message: String,
    },
}
