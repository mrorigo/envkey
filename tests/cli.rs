use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn auth_token(home: &std::path::Path) -> String {
    let mut auth = Command::cargo_bin("envkey").expect("bin");
    let output = auth
        .env("HOME", home)
        .env("ENVKEY_AUTH_PASSWORD", "pw1")
        .args(["auth"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let line = String::from_utf8(output).expect("utf8");
    let token = line
        .trim()
        .strip_prefix("export ENVKEY_SESSION='")
        .and_then(|s| s.strip_suffix('\''))
        .expect("token format");
    token.to_string()
}

#[test]
fn add_and_run_injects_variable() {
    let home = tempdir().expect("tempdir");
    let session = auth_token(home.path());

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "dev", "API_KEY", "abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated key 'API_KEY' in profile 'dev'",
        ));

    let mut run = Command::cargo_bin("envkey").expect("bin");
    run.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args([
            "run",
            "--profile",
            "dev",
            "--",
            "sh",
            "-c",
            "printf %s \"$API_KEY\"",
        ])
        .assert()
        .success()
        .stdout("abc123");
}

#[test]
fn commands_auto_auth_when_session_missing() {
    let home = tempdir().expect("tempdir");

    let mut profiles = Command::cargo_bin("envkey").expect("bin");
    profiles
        .env("HOME", home.path())
        .env("ENVKEY_AUTH_PASSWORD", "pw1")
        .args(["profiles"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn profiles_lists_available_profiles_sorted() {
    let home = tempdir().expect("tempdir");
    let session = auth_token(home.path());

    let mut add1 = Command::cargo_bin("envkey").expect("bin");
    add1.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "prod", "A", "1"])
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("envkey").expect("bin");
    add2.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "dev", "B", "2"])
        .assert()
        .success();

    let mut profiles = Command::cargo_bin("envkey").expect("bin");
    profiles
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["profiles"])
        .assert()
        .success()
        .stdout("dev\nprod\n");
}

#[test]
fn env_output_is_eval_safe_with_single_quotes() {
    let home = tempdir().expect("tempdir");
    let session = auth_token(home.path());

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "dev", "ANTHROPIC_API_KEY", "sk'ant"])
        .assert()
        .success();

    let mut shell = Command::new("sh");
    let envkey_bin = env!("CARGO_BIN_EXE_envkey");
    shell
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .arg("-c")
        .arg(format!(
            "eval \"$({} env --profile dev)\"; printf %s \"$ANTHROPIC_API_KEY\"",
            envkey_bin
        ))
        .assert()
        .success()
        .stdout("sk'ant");
}

#[test]
fn profile_and_key_remove_with_yes() {
    let home = tempdir().expect("tempdir");
    let session = auth_token(home.path());

    let mut add_a = Command::cargo_bin("envkey").expect("bin");
    add_a
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "dev", "OPENAI_API_KEY", "sk-1"])
        .assert()
        .success();

    let mut add_b = Command::cargo_bin("envkey").expect("bin");
    add_b
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["add", "--profile", "dev", "ANTHROPIC_API_KEY", "sk-ant-1"])
        .assert()
        .success();

    let mut key_rm = Command::cargo_bin("envkey").expect("bin");
    key_rm
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["key-rm", "--profile", "dev", "OPENAI_API_KEY", "-y"])
        .assert()
        .success();

    let mut env_cmd = Command::cargo_bin("envkey").expect("bin");
    env_cmd
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["env", "--profile", "dev"])
        .assert()
        .success()
        .stdout("export ANTHROPIC_API_KEY='sk-ant-1'\n");

    let mut profile_rm = Command::cargo_bin("envkey").expect("bin");
    profile_rm
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["profile-rm", "--profile", "dev", "-y"])
        .assert()
        .success();

    let mut profiles = Command::cargo_bin("envkey").expect("bin");
    profiles
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["profiles"])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn lock_and_logout_invalidate_session() {
    let home = tempdir().expect("tempdir");
    let session = auth_token(home.path());

    let mut lock = Command::cargo_bin("envkey").expect("bin");
    lock.env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["lock"])
        .assert()
        .success();

    let mut profiles = Command::cargo_bin("envkey").expect("bin");
    profiles
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["profiles"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SESSION_LOCKED"));

    let mut logout = Command::cargo_bin("envkey").expect("bin");
    logout
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["logout"])
        .assert()
        .success();

    let mut profiles2 = Command::cargo_bin("envkey").expect("bin");
    profiles2
        .env("HOME", home.path())
        .env("ENVKEY_SESSION", &session)
        .args(["profiles"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SESSION_MISSING"));
}
