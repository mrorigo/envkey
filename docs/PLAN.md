# envkey Implementation Plan

## Goal
Build `envkey` (with `ek` shorthand) as a secure Rust CLI for encrypted secret storage and process-level environment injection.

## Scope (v1)
- Encrypted local vault at `~/.envkey/vault.db`.
- CLI commands:
  - `envkey add --profile <name> <key> [value]`
  - `envkey run --profile <name> -- <command>`
- Strong crypto defaults (Argon2id + AES-256-GCM).
- No secret persistence in shell env/history.
- Cross-platform support: macOS + Linux first.

## Architecture Decisions
- Language/runtime: Rust stable.
- CLI parser: `clap` derive.
- Serialization: `serde` + `serde_json`.
- KDF: `argon2` (Argon2id with tunable params).
- AEAD: `aes-gcm` (AES-256-GCM).
- Secret memory handling: `secrecy` + `zeroize`.
- Atomic writes: write temp file + `rename`.

## Work Breakdown

### Phase 0: Repository Setup
- Create crate layout:
  - `src/main.rs`
  - `src/cli.rs`
  - `src/vault/mod.rs`
  - `src/vault/crypto.rs`
  - `src/vault/store.rs`
  - `src/commands/add.rs`
  - `src/commands/run.rs`
  - `tests/` (integration tests)
- Add dependencies and lint config.
- Add binary alias plan:
  - Primary binary: `envkey`
  - Secondary alias: `ek` (symlink/install alias in release packaging).

Exit criteria:
- `cargo check` passes.
- `envkey --help` shows command skeleton.

### Phase 1: Vault Data Model + Storage Layer
- Implement `Vault` and `Profile` structs with serde support.
- Implement path resolver for `~/.envkey/vault.db`.
- Implement safe file IO:
  - create directory with restrictive permissions
  - read existing file or initialize empty vault
  - atomic write with fsync where applicable
- Add schema version field for forward compatibility.

Exit criteria:
- Unit tests for read/write roundtrip.
- File permissions validated in tests (platform-conditional).

### Phase 2: Cryptography Layer
- Implement key derivation:
  - random salt per vault
  - Argon2id params configurable by constants
- Implement encrypt/decrypt:
  - random nonce per write
  - AES-256-GCM sealed payload
- Implement clear error taxonomy:
  - bad password
  - corrupt vault
  - unsupported schema version
- Ensure sensitive buffers are zeroized after use.

Exit criteria:
- Unit tests for:
  - encrypt/decrypt roundtrip
  - wrong password failure
  - tamper detection
- Bench check confirms decrypt + parse stays within target budget for normal vault sizes.

### Phase 3: `add` Command
- Implement `envkey add --profile <name> <key> [value]`.
- If `value` is omitted, prompt with no-echo input.
- Load/decrypt vault, upsert key, re-encrypt, atomic write.
- Output minimal success message (no secret echoes).

Exit criteria:
- Integration tests:
  - add new key
  - overwrite existing key
  - add into new profile
- No secret value appears in stdout/stderr.

### Phase 4: `run` Command
- Implement `envkey run --profile <name> -- <command>`.
- Decrypt selected profile and inject vars into child process only.
- Forward stdin/stdout/stderr; return child exit code.
- Validate error handling for missing profile and missing command.

Exit criteria:
- Integration tests:
  - command receives injected env var
  - parent process env remains unchanged
  - child exit codes propagate correctly

### Phase 5: UX, Hardening, and Packaging
- Add consistent CLI errors and help text.
- Add optional keyring cache for derived key/session (if enabled by feature flag).
- Add shell completion generation.
- Prepare release artifacts:
  - binary naming (`envkey` + `ek`)
  - install docs and checksum generation.

Exit criteria:
- `cargo test` green.
- `cargo clippy -- -D warnings` green.
- Release build runs on macOS/Linux CI matrix.

## Testing Strategy
- Unit tests:
  - crypto correctness, serialization, path handling.
- Integration tests:
  - end-to-end `add` + `run` flows via `assert_cmd`.
- Security regression tests:
  - reject modified ciphertext
  - reject nonce/salt mismatch
- Performance check:
  - measure run-path overhead on representative machine; enforce `<10ms` target for injection path excluding process startup noise.

## Risks and Mitigations
- Argon2 parameters too slow on low-spec machines:
  - Mitigate with calibrated defaults and documented override.
- Secret exposure through logs/panics:
  - Mitigate with strict formatting policy; avoid `Debug` for secret types.
- Cross-platform file permission differences:
  - Mitigate with platform-specific permission handling and tests.
- `ek` alias availability on user PATH:
  - Mitigate by shipping explicit install instructions and package-level alias.

## Concrete Task List
1. Scaffold module structure and CLI wiring.
2. Implement vault schema + versioning.
3. Implement encrypted store read/write with atomic persistence.
4. Implement password prompt and secret-safe input handling.
5. Implement `add` command end-to-end.
6. Implement `run` command end-to-end.
7. Add comprehensive tests and golden error cases.
8. Add CI jobs for test/lint/build matrix.
9. Add release packaging for `envkey` and `ek`.
10. Final security review pass and docs polish.

## Definition of Done
- All scoped commands implemented and tested.
- Security requirements from PRD validated by tests and code review.
- Documentation updated for install, usage, and threat model boundaries.
- A tagged release candidate is buildable and reproducible.
