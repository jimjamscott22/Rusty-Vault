# Rusty_Vault — Claude Code Starter Prompt

> Attach this prompt alongside `Rusty_Vault_Spec.md` when starting your Claude Code session.

---

## Prompt

I am building a project called **Rusty_Vault** — a self-hosted, multi-machine CLI secrets vault written in Rust. I have attached a full spec sheet (`Rusty_Vault_Spec.md`) that covers the architecture, crypto design, data model, server API, CLI commands, project structure, and a 5-phase implementation plan. Please read it carefully before writing any code.

### Your task for this session: Phase 1 + Phase 2

Implement **Phase 1 (Project Scaffold & Crypto Primitives)** and **Phase 2 (CLI Skeleton & Config)** from the spec. Do not start on the server or networking yet — get the foundation right first.

#### Specifically, by the end of this session I want:

1. **A Cargo workspace** at the root with two member crates: `rv-server` and `rv-client`. Set up both `Cargo.toml` files with the correct dependencies from the spec. The server crate can remain mostly empty for now — just a valid crate with a `main.rs` stub.

2. **`rv-client/src/crypto.rs`** implementing:
   - `derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32]` using Argon2id
   - `encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8>` using AES-256-GCM — returns `nonce || ciphertext` (random 96-bit nonce prepended)
   - `decrypt(payload: &[u8], key: &[u8; 32]) -> anyhow::Result<Vec<u8>>` — splits nonce from payload, decrypts, returns plaintext
   - Unit tests that verify a full encrypt → decrypt roundtrip on at least two inputs: a short string and a simulated `.env` file content

3. **`rv-client/src/config.rs`** implementing:
   - A `Config` struct (server_url, auth_token, kdf_salt as base64)
   - `Config::load() -> anyhow::Result<Config>` — reads from `~/.config/rusty_vault/config.toml`
   - `Config::save(&self) -> anyhow::Result<()>` — writes to the same path, creating directories if needed

4. **`rv-client/src/main.rs`** with a full `clap` command structure for all commands from the spec: `init`, `push`, `pull`, `list`, `delete`, `ping`. Commands other than `init` can print a `"Not yet implemented"` placeholder for now.

5. **`init` command** fully implemented:
   - Prompt the user interactively for server URL and auth token
   - Generate a cryptographically random 32-byte KDF salt
   - Save config to `~/.config/rusty_vault/config.toml`
   - Print a clear success message with the config path

6. **`cargo test` must pass** with all crypto unit tests green before you consider Phase 1 done.

---

### Code quality expectations

- Use a modular file structure exactly as described in the spec (`commands/`, `crypto.rs`, `config.rs`, `client.rs`)
- Use `anyhow` for error handling throughout — no unwraps in library code, only in `main` where appropriate
- Add doc comments (`///`) to all public functions explaining what they do, their parameters, and what errors they return
- Keep the code clean and readable — I am a CS student and will be studying this codebase to learn from it, so clarity matters as much as correctness
- Do not over-engineer — prefer simple, explicit code over clever abstractions

### Rust version & toolchain

- Use the **stable** Rust toolchain
- Target edition: **2021**
- The server will eventually be cross-compiled for a Raspberry Pi (`aarch64-unknown-linux-gnu`), so avoid crates that have poor cross-compilation support

### When you are done

- Run `cargo test` and show me the output
- Run `cargo clippy` and fix any warnings
- Give me a brief summary of what was built, any design decisions you made that weren't in the spec, and what Phase 3 (the server) will need to tackle next

---

### Reference: Key crates to use

| Crate       | Purpose                              |
|-------------|--------------------------------------|
| `argon2`    | Argon2id key derivation              |
| `aes-gcm`   | AES-256-GCM authenticated encryption |
| `rand`      | Cryptographically secure nonce gen   |
| `clap`      | CLI argument parsing (derive feature)|
| `anyhow`    | Ergonomic error handling             |
| `serde`     | Serialization                        |
| `toml`      | Config file parsing                  |
| `dirs`      | XDG config directory resolution      |
| `rpassword` | Secure passphrase prompt (no echo)   |
| `base64`    | Encoding the KDF salt for config     |

---

> Once Phase 1 and 2 are complete and tests are green, start a new Claude Code session for Phase 3 (the axum server) and attach the spec sheet again along with the completed source files for context.
