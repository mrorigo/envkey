use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn add_and_run_injects_variable() {
    let home = tempdir().expect("tempdir");

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "API_KEY", "abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Updated key 'API_KEY' in profile 'dev'",
        ));

    let mut run = Command::cargo_bin("envkey").expect("bin");
    run.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
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
fn run_fails_for_missing_profile() {
    let home = tempdir().expect("tempdir");

    let mut run = Command::cargo_bin("envkey").expect("bin");
    run.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["run", "--profile", "missing", "--", "echo", "ok"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Profile not found: missing"));
}

#[test]
fn run_propagates_exit_code() {
    let home = tempdir().expect("tempdir");

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "A", "1"])
        .assert()
        .success();

    let mut run = Command::cargo_bin("envkey").expect("bin");
    run.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["run", "--profile", "dev", "--", "sh", "-c", "exit 7"])
        .assert()
        .code(7);
}

#[test]
fn parent_environment_is_not_modified() {
    let home = tempdir().expect("tempdir");

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "API_KEY", "vault"])
        .assert()
        .success();

    let mut run = Command::cargo_bin("envkey").expect("bin");
    run.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .env("API_KEY", "parent")
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
        .stdout("vault");
}

#[test]
fn env_outputs_export_lines() {
    let home = tempdir().expect("tempdir");

    let mut add1 = Command::cargo_bin("envkey").expect("bin");
    add1.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "OPENAI_API_KEY", "sk-123"])
        .assert()
        .success();

    let mut add2 = Command::cargo_bin("envkey").expect("bin");
    add2.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "REPLICATE_API_TOKEN", "r8_456"])
        .assert()
        .success();

    let mut env_cmd = Command::cargo_bin("envkey").expect("bin");
    env_cmd
        .env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["env", "--profile", "dev"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export OPENAI_API_KEY='sk-123'\n"))
        .stdout(predicate::str::contains(
            "export REPLICATE_API_TOKEN='r8_456'\n",
        ));
}

#[test]
fn env_output_is_eval_safe_with_single_quotes() {
    let home = tempdir().expect("tempdir");

    let mut add = Command::cargo_bin("envkey").expect("bin");
    add.env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .args(["add", "--profile", "dev", "ANTHROPIC_API_KEY", "sk'ant"])
        .assert()
        .success();

    let mut shell = Command::new("sh");
    let envkey_bin = env!("CARGO_BIN_EXE_envkey");
    shell
        .env("HOME", home.path())
        .env("ENVKEY_MASTER_PASSWORD", "pw1")
        .arg("-c")
        .arg(format!(
            "eval \"$({} env --profile dev)\"; printf %s \"$ANTHROPIC_API_KEY\"",
            envkey_bin
        ))
        .assert()
        .success()
        .stdout("sk'ant");
}
