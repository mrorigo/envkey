# envkey

`envkey` (`ek`) is a local-first secrets CLI for development workflows.

It encrypts secrets at rest and uses a local daemon-backed session model so you authenticate once and run multiple commands securely without putting your master password in shell environment variables.

## Features
- Encrypted vault at rest (`~/.envkey/vault.db`) using Argon2id + AES-256-GCM.
- Profile-based secret organization (`dev`, `staging`, `prod`, etc).
- Daemon-backed ephemeral sessions (`envkeyd`) with idle/max lifetime controls.
- Secure command execution with injected environment variables (`ek run`).
- Shell export output for intentional in-shell loading (`ek env`).
- Profile/key lifecycle operations (list/add/remove).

## Architecture
`envkey` has two runtime components:
- `ek` CLI: user-facing command parser and daemon client.
- `envkeyd` daemon: local Unix-socket server that holds unlocked key material in memory for active sessions.

Session model:
1. `ek auth` prompts once for the master password.
2. Daemon validates/decrypts vault and creates a session token.
3. CLI prints an export line for `ENVKEY_SESSION`.
4. Subsequent commands use `ENVKEY_SESSION` to authorize vault operations.

## Installation
### From source
```sh
cargo build --release
```

Binary output:
- `target/release/envkey`

Optional shorthand:
- create a symlink/alias `ek -> envkey`

## Quick Start
### 1. Authenticate once
```sh
eval "$(ek auth)"
```

### 2. Add secrets
```sh
ek add --profile dev OPENAI_API_KEY sk-...
ek add --profile dev ANTHROPIC_API_KEY sk-ant-...
```

### 3. Use secrets in a subprocess
```sh
ek run --profile dev -- node app.js
```

### 4. (Optional) export to current shell
```sh
eval "$(ek env --profile dev)"
```

## Command Reference
### Session/Auth
- `ek auth`
  - Prompts for master password, starts daemon if needed, prints:
  - `export ENVKEY_SESSION='...'`
- `ek status`
  - Shows daemon/session status.
- `ek lock`
  - Locks current session.
- `ek logout`
  - Revokes current session.

### Vault Operations
- `ek add --profile <name> <KEY> [VALUE]`
  - Add/update secret. If `VALUE` is omitted, prompts securely.
- `ek run --profile <name> -- <command ...>`
  - Runs command with profile vars injected into child process env.
- `ek env --profile <name>`
  - Prints shell export lines (`export KEY='value'`) with safe quoting.
- `ek profiles`
  - Lists all profiles.
- `ek key-rm --profile <name> <KEY> [-y]`
  - Removes one key (`-y` skips confirmation).
- `ek profile-rm --profile <name> [-y]`
  - Removes profile and all keys (`-y` skips confirmation).

## Security Model
### What envkey protects
- Secrets are encrypted at rest.
- Master password is not required for every command after session auth.
- Operational commands require `ENVKEY_SESSION` instead of `ENVKEY_MASTER_PASSWORD`.
- Sessions can be locked/logged out and expire automatically.

### Important tradeoffs
- `ek env` intentionally prints plaintext exports; use only when needed.
- A valid `ENVKEY_SESSION` token is a capability for the session lifetime.
- Local root/host compromise is out of scope.

## Runtime Paths
- Vault file: `~/.envkey/vault.db`
- Runtime dir: `~/.envkey/run`
- Daemon socket: `~/.envkey/run/envkeyd.sock`

## Environment Variables
- `ENVKEY_SESSION`
  - Active session token used by operational commands.
- `ENVKEY_AUTH_PASSWORD`
  - Optional non-interactive input for `ek auth` (primarily for tests/automation).

## Docker / Containers
Recommended:
- Run `ek` and `envkeyd` in the same container.

Notes:
- Session state resets when container restarts.
- If splitting CLI/daemon across containers, share runtime socket path and maintain strict UID/permission controls.

## Development
### Test and lint
```sh
cargo test
cargo clippy -- -D warnings
cargo fmt
```

### Daemon design doc
See [docs/DAEMON_PLAN.md](docs/DAEMON_PLAN.md) for detailed design, protocol, and rollout rationale.

## Roadmap
- Windows support via IPC backend abstraction (named pipes).
- Additional session policies (fresh-auth gates for sensitive operations).
- Optional shell integration helpers for session lifecycle.

## License
MIT
