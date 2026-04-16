This is a compelling project. Building a secure environment injector in Rust allows you to leverage zero-cost abstractions and memory safety, which are paramount for a tool handling sensitive credentials.

---

## Product Requirements Document (PRD)

### 1. Purpose
**envkey** (shorthand: `ek`) is a CLI utility designed to eliminate "secret leakage" caused by hardcoding environment variables, messy `.env` files, or clipboard history. It provides a secure, encrypted vault for credentials and injects them directly into a child process's memory.

### 2. User Personas
* **Developers:** Who need to switch between `staging`, `dev`, and `prod` keys without polluting their `.bash_history`.
* **SRE/DevOps:** Who require a minimal-footprint tool to run scripts with ephemeral secrets.

### 3. Core Features
* **Encrypted Storage:** AES-256-GCM encryption for all keys at rest, keyed by a user-defined Master Password.
* **Profile Management:** Group keys into logical sets (e.g., `work`, `side-project`, `prod`).
* **Safe Injection:** Execute commands as child processes with secrets injected directly into their environment block.
* **Zero Shell Pollution:** Secrets never exist in the parent shell's environment variables.

### 4. Non-Functional Requirements
* **Performance:** Injection overhead should be $<10ms$.
* **Security:** Memory should be zeroed out after use; no plaintext keys written to disk.
* **Portability:** Single static binary (typical of Rust).

---

## Technical Specification

### 1. Data Architecture
The store will be a single encrypted file (e.g., `~/.envkey/vault.db`). 

**Storage Schema (JSON/Bincode before encryption):**
```rust
struct Vault {
    salt: [u8; 32],
    nonce: [u8; 12],
    profiles: HashMap<String, Profile>,
}

struct Profile {
    vars: HashMap<String, String>, // Key: VAR_NAME, Value: Secret
}
```

### 2. Cryptographic Stack
* **KDF:** Argon2id to derive a 256-bit key from the Master Password.
* **Encryption:** `AES-256-GCM` (using the `aes-gcm` crate).
* **Memory Safety:** Use the `secrecy` crate to wrap keys in `SecretString`, ensuring they are zeroed out of memory and not accidentally logged.

### 3. Command Implementation

#### `envkey add --profile <name> <key> [value]` (or `ek add ...`)
1. Prompt for Master Password (if not cached in `keyring`).
2. Load and decrypt the vault.
3. Update the `HashMap` for the specified profile.
4. Re-encrypt and atomic-write to disk.

#### `envkey run --profile <name> -- <command>` (or `ek run ...`)
1. Decrypt the requested profile.
2. Use `std::process::Command`.
3. Call `.envs(&profile_vars)` on the command builder.
4. `spawn()` the child process and inherit `stdin/stdout/stderr`.

---

## Implementation (Proof of Concept)

Here is a foundational implementation of the `run` functionality and the encryption logic.

### Cargo.toml
```toml
[dependencies]
clap = { version = "4.0", features = ["derive"] }
aes-gcm = "0.10"
argon2 = "0.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
secrecy = "0.8"
zeroize = "1.5"
```

### Main Logic (src/main.rs)
```rust
use clap::{Parser, Subcommand};
use std::process::{Command, ExitStatus};
use std::collections::HashMap;

#[derive(Parser)]
#[command(name = "envkey")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a key to a profile
    Add {
        #[arg(short, long)]
        profile: String,
        key: String,
        value: Option<String>,
    },
    /// Run a command with injected secrets
    Run {
        #[arg(short, long)]
        profile: String,
        #[arg(last = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run { profile, args } => {
            if args.is_empty() {
                eprintln!("Error: No command provided.");
                return;
            }

            // 1. In a real app, you'd decrypt the vault here.
            // Mocking the decrypted data for this example:
            let mut secrets = HashMap::new();
            if profile == "dev" {
                secrets.insert("STRIPE_API_KEY", "sk_test_4eC39HqLyjWDarjtT1zdp7dc");
            }

            // 2. Execute the child process
            let status = execute_injected_command(&args[0], &args[1..], secrets);
            
            std::process::exit(status.code().unwrap_or(1));
        }
        Commands::Add { .. } => {
            todo!("Implement Argon2 + AES-GCM storage flow");
        }
    }
}

fn execute_injected_command(cmd: &str, args: &[String], envs: HashMap<&str, &str>) -> ExitStatus {
    Command::new(cmd)
        .args(args)
        .envs(envs) // Secrets injected here
        .status()
        .expect("Failed to execute command")
}
```

### Security Considerations for Implementation
1.  **Master Password:** Do not store it. Use the OS native keyring (via `keyring-rs`) to store the derived key temporarily if you want a "session" feel.
2.  **Ptrace/Debugging:** On Linux, an attacker with enough privileges could potentially attach to the process and read the environment block. Rust cannot prevent this, but keeping the lifetime of the secrets in the parent process short helps.
3.  **Swap:** Consider using `mlock` (via `nix` crate) to prevent the sensitive parts of the vault from being swapped to disk while the app is running.

