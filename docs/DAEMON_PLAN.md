# envkey Daemon Plan

## 1. Objective
Implement secure ephemeral sessions so users/agents authenticate once per session and can run multiple `ek` commands without exposing the master password in environment variables.

Primary deliverable:
- New local daemon (`envkeyd`) that holds unlocked key material in memory with strict TTL/idle limits.

Secondary deliverable:
- CLI auth/session commands (`auth`, `status`, `lock`, `logout`) and migration away from `ENVKEY_MASTER_PASSWORD` fallback for normal flows.

## 2. Scope and Non-Goals
In scope (v1):
- macOS/Linux support using Unix domain sockets.
- Session-based unlock and command authorization.
- IPC abstraction boundary designed for future Windows support.
- CLI updates to consume daemon sessions.

Out of scope (v1):
- Windows runtime support (implementation deferred).
- Remote daemon access over TCP.
- Multi-user shared daemon.
- Hardware-backed key storage.

## 3. Security Goals
- Master password is never persisted.
- Master password is not passed via environment variables.
- Unlocked key material exists only in daemon memory.
- Sessions expire automatically and can be explicitly revoked.
- Only same-user local processes can use sessions.
- Sensitive buffers are zeroized after use.

## 4. Threat Model
Assumptions:
- Local machine is not fully compromised at kernel/root level.
- User account isolation is meaningful.

Threats addressed:
- Secret leakage via shell env (`ENVKEY_MASTER_PASSWORD`).
- Repeated password prompts for agent workflows.
- Unauthorized same-host users attaching to session endpoint.

Threats not fully addressed:
- Root-level attackers.
- Memory scraping after full host compromise.
- User intentionally exporting plaintext with `ek env`.

## 5. High-Level Architecture
Components:
- `ek` (CLI frontend): user-facing command parser + request client.
- `envkeyd` (daemon): in-memory key/session manager + vault operation engine.
- Vault store: existing encrypted file at `~/.envkey/vault.db`.

Flow:
1. User runs `ek auth`.
2. CLI prompts once for master password.
3. CLI sends unlock request to daemon via local IPC.
4. Daemon derives key, validates decrypt, stores key in memory, issues opaque session token.
5. CLI prints shell export command for token (or writes to shell integration output).
6. Subsequent `ek` commands call daemon with `ENVKEY_SESSION` token.
7. Daemon authorizes, executes operation, returns result.

## 6. IPC Abstraction Strategy
Define a transport trait now, implement Unix socket backend only for v1.

Proposed interface:
- `IpcServer`:
  - `bind(endpoint)`
  - `accept()`
  - `recv()` / `send()`
- `IpcClient`:
  - `connect(endpoint)`
  - `request(response)`

Endpoint abstraction:
- `enum Endpoint { UnixSocket(PathBuf), NamedPipe(String) }`

v1 endpoint:
- `~/.envkey/run/envkeyd.sock`

Future Windows backend:
- Named pipe implementation behind the same trait.

## 7. Daemon Process Lifecycle
Startup:
- Lazy-start daemon on first `ek auth` (or first session-required command if absent).
- Create runtime dir `~/.envkey/run` with permissions `0700`.
- Create socket with restrictive permissions.

Runtime:
- Maintain in-memory sessions map.
- Enforce max session lifetime + idle timeout.
- Periodic sweeper task to expire sessions.

Shutdown:
- Graceful on signal.
- Zeroize all in-memory key material.
- Remove socket file.

## 8. Session Model
Session record fields:
- `session_id: [u8; 32]` random token, base64/hex encoded externally.
- `created_at`, `last_used_at`, `expires_at`.
- `key_material` (wrapped + zeroizable type).
- `state: Active | Locked | Expired`.
- `policy`:
  - `max_lifetime`
  - `idle_timeout`
  - `require_reauth_for_destructive` (future toggle)

Defaults:
- Max lifetime: 60 minutes.
- Idle timeout: 15 minutes.

Token handling:
- CLI reads token from `ENVKEY_SESSION`.
- Token is opaque capability string (no embedded data).

## 9. Command Surface Changes
New commands:
- `ek auth`
  - Prompts for master password.
  - Starts daemon if needed.
  - Creates/refreshes session.
  - Prints: `export ENVKEY_SESSION='...'`
- `ek status`
  - Shows daemon reachability and session validity.
- `ek lock`
  - Locks current session (keeps daemon alive).
- `ek logout`
  - Revokes current session immediately.

Existing commands behavior:
- `add`, `run`, `env`, `profiles`, `key-rm`, `profile-rm` require valid session token.
- If no valid session: clear error with remediation (`run ek auth`).

Deprecation:
- `ENVKEY_MASTER_PASSWORD` support moved to explicit compatibility mode and then removed.

## 10. IPC Protocol (v1)
Encoding:
- JSON messages over framed stream (length-prefix u32).

Request envelope:
```json
{ "id": "uuid", "method": "run", "session": "token", "params": { ... } }
```

Response envelope:
```json
{ "id": "uuid", "ok": true, "result": { ... } }
```
or
```json
{ "id": "uuid", "ok": false, "error": { "code": "SESSION_EXPIRED", "message": "..." } }
```

Initial methods:
- `auth.unlock`
- `auth.status`
- `auth.lock`
- `auth.logout`
- `vault.add`
- `vault.run`
- `vault.env`
- `vault.profiles`
- `vault.key_remove`
- `vault.profile_remove`

Error codes:
- `UNAUTHORIZED`
- `SESSION_MISSING`
- `SESSION_EXPIRED`
- `SESSION_LOCKED`
- `BAD_REQUEST`
- `PROFILE_NOT_FOUND`
- `KEY_NOT_FOUND`
- `VAULT_ERROR`

## 11. Access Control Details
Unix socket protections:
- Runtime dir `0700`
- Socket file owner-only access
- Verify peer credentials (UID) where available

Session protections:
- Strong random token generation (CSPRNG)
- Constant-time token comparison
- Rate-limit failed auth/session attempts

Data handling:
- Zeroize plaintext and derived key buffers
- Avoid logging secrets/tokens
- Redact sensitive fields in diagnostics

## 12. UX and Agent Workflow
Agent-friendly flow:
1. Agent runs `ek status`.
2. If no active session, asks user once to run `ek auth`.
3. User runs `ek auth` and exports returned token.
4. Agent executes multiple `ek` commands without password prompts.

Human-friendly messaging:
- On expired session: `Session expired. Re-authenticate with: ek auth`.
- On missing token: `No ENVKEY_SESSION set.`

## 13. Docker and Container Guidance
Supported model:
- Daemon and `ek` run in same container namespace.

Notes:
- If daemon and CLI are split across containers, share runtime dir volume and preserve uid permissions.
- Session lifetime resets on container restart.
- Do not mount runtime socket to broader scopes unless intentional.

## 14. Implementation Plan

Phase 1: Foundations
- Add daemon crate/module skeleton.
- Add IPC trait and Unix socket implementation.
- Add shared request/response schema.

Phase 2: Session/Auth
- Implement `auth.unlock`, `status`, `lock`, `logout`.
- Implement session store + sweeper.
- Add CLI commands `auth/status/lock/logout`.

Phase 3: Vault RPC Migration
- Move existing vault operations behind daemon RPC methods.
- Update CLI commands to call daemon instead of local decrypt path.

Phase 4: Hardening
- Peer credential checks.
- Token comparison hardening.
- Structured error mapping.
- Add lock on inactivity and signal-safe cleanup.

Phase 5: Migration Cleanup
- Mark `ENVKEY_MASTER_PASSWORD` path deprecated.
- Remove fallback after compatibility window.
- Update README/docs/examples.

## 15. Testing Plan
Unit tests:
- Session expiration and idle timeout logic.
- Token generation/validation and constant-time compare wrappers.
- Request/response serialization.

Integration tests:
- `ek auth` then `ek add/run/env/profiles` success path.
- Session expiry rejection.
- `lock/logout` behavior.
- `key-rm/profile-rm` via daemon with `-y` and confirm paths.

Security tests:
- Socket permission checks.
- Cross-user access rejection simulation where feasible.
- Ensure sensitive values absent from logs/errors.

## 16. Operational Observability
Minimal diagnostics (non-sensitive):
- daemon startup/shutdown
- active session count
- method-level success/failure counters (no params)

Debug mode:
- Explicit opt-in env var for verbose logs.
- Mandatory redaction path for secrets/tokens.

## 17. Open Decisions
- Single session vs multi-session support per user (recommend multi-session with limits).
- Default TTL values (recommend 60m max / 15m idle).
- Whether destructive ops should enforce fresh auth in v1 or v2.
- Whether `ek auth` should print export line vs shell integration helper command.

## 18. Definition of Done
- `ek auth/status/lock/logout` implemented and documented.
- Existing commands operate through daemon sessions.
- No normal-path reliance on `ENVKEY_MASTER_PASSWORD`.
- Test suite covers auth lifecycle and vault operations with session gating.
- Security review checklist completed for local IPC/session handling.
