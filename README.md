# envkey

A secure secrets-injection CLI that was apparently so hard to build it took:
- one idea pasted into Gemini,
- one PRD,
- one pass through Codex,
- and less time than a mediocre coffee break.

Total effort: ~20 minutes.

Enterprise security theater effort saved: incalculable.

## What it does
- Encrypts secrets at rest (`~/.envkey/vault.db`) using Argon2id + AES-256-GCM.
- Stores secrets by profile (`dev`, `prod`, etc).
- Injects secrets into child processes without polluting parent shell env.
- Prints shell exports when you want to `eval` into current shell.

## Install
```sh
cargo build --release
# binary: target/release/envkey
# optional shorthand: ek (symlink envkey -> ek)
```

## Usage

### Add/update a secret
```sh
envkey add --profile dev OPENAI_API_KEY sk-...
# or
ek add --profile dev OPENAI_API_KEY sk-...
```

### Run a command with injected env vars
```sh
envkey run --profile dev -- node app.js
# or
ek run --profile dev -- node app.js
```

### Export into current shell
```sh
eval "$(ek env --profile dev)"
```

Produces lines like:
```sh
export OPENAI_API_KEY='sk-...'
export ANTHROPIC_API_KEY='sk-ant-...'
export REPLICATE_API_TOKEN='r8_...'
```

## Security model (the serious part)
- Secrets are encrypted at rest.
- Master password is required to decrypt vault data.
- `run` injects vars into child process only.
- `env` intentionally prints plaintext exports for shell consumption. Use it only when you accept that tradeoff.

## Dev
```sh
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## Why this exists
Because setting up “proper secret management” for local dev is usually treated like a six-month transformation program.

It is not.
