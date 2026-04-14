# Rusty_Vault — Spec Sheet & Implementation Plan

---

## Overview

**Rusty_Vault** is a self-hosted, multi-machine CLI secrets vault built in Rust. It is designed to securely store sensitive files — primarily `.env` files, SSH keys, API key bundles, and TLS certificates — with all encryption happening client-side. The vault storage lives on a Raspberry Pi accessible over a Tailscale network, so secrets are reachable from any trusted machine without exposing anything to the public internet.

The server is cryptographically blind: it stores and serves encrypted blobs but never has access to the plaintext or the encryption key. All crypto happens on the client.

---

## Goals

- Encrypt secret files client-side and push them to a Pi-hosted server
- Retrieve and decrypt those files from any machine on the Tailscale network
- Organize secrets by project namespace for easy management
- Keep the architecture simple, auditable, and educational
- Serve as both a daily-use personal tool and a strong Rust portfolio project

## Non-Goals (for now)

- Multi-user access control (single-user tool)
- A GUI or web dashboard
- Replacing Vaultwarden for passwords/credentials
- Public internet exposure

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Tailscale Network                  │
│                                                     │
│  ┌──────────────────┐       ┌────────────────────┐  │
│  │  Client Machine  │       │  Raspberry Pi      │  │
│  │  (Laptop, etc.)  │       │  (Server)          │  │
│  │                  │       │                    │  │
│  │  rv-client CLI   │──────▶│  rv-server (axum)  │  │
│  │                  │ HTTPS │                    │  │
│  │  Encrypts /      │       │  Stores encrypted  │  │
│  │  Decrypts        │       │  blobs + metadata  │  │
│  │  locally         │       │  (SQLite + disk)   │  │
│  └──────────────────┘       └────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Key Design Principle

The **encryption key never leaves the client machine.** The server receives and stores only ciphertext. Even if the Pi is compromised, secrets remain protected by the client's master passphrase.

---

## Crypto Stack

| Primitive        | Algorithm    | Crate         | Purpose                              |
|------------------|--------------|---------------|--------------------------------------|
| Key Derivation   | Argon2id     | `argon2`      | Derives 256-bit key from passphrase  |
| Encryption       | AES-256-GCM  | `aes-gcm`     | Authenticated encryption of files    |
| Nonce            | Random 96-bit| `rand`        | Unique per encryption operation      |

### Encryption Flow (Client)

```
Master Passphrase
       │
       ▼
  Argon2id KDF ──── Salt (stored in config) ────▶ 256-bit Key
       │
       ▼
  AES-256-GCM
       │
  Random Nonce (96-bit)
       │
       ▼
  Ciphertext = Encrypt(plaintext, key, nonce)
       │
       ▼
  Payload = nonce || ciphertext  ──▶  POST to server
```

### Decryption Flow (Client)

```
  Payload (fetched from server)
       │
       ▼
  Split: nonce (first 12 bytes) || ciphertext (rest)
       │
       ▼
  Argon2id KDF (passphrase + stored salt) ──▶ 256-bit Key
       │
       ▼
  AES-256-GCM Decrypt(ciphertext, key, nonce) ──▶ Plaintext file
```

---

## Data Model

### Namespace

A logical grouping for secrets, typically by project or category.

Examples: `myapp`, `homelab`, `ssh-keys`, `certs`

### Secret

A single encrypted file within a namespace.

| Field        | Type      | Description                          |
|--------------|-----------|--------------------------------------|
| `id`         | UUID      | Unique identifier                    |
| `namespace`  | String    | Grouping (e.g., `myapp`)             |
| `name`       | String    | Identifier within namespace (e.g., `.env`) |
| `blob_path`  | String    | Path to encrypted file on disk       |
| `created_at` | Timestamp | When the secret was first pushed     |
| `updated_at` | Timestamp | When the secret was last updated     |
| `size_bytes` | Integer   | Size of encrypted blob               |

### On-disk Storage (Pi)

```
~/.rusty_vault/
├── vault.db          # SQLite metadata
└── blobs/
    ├── <uuid-1>      # Raw encrypted blob (nonce || ciphertext)
    ├── <uuid-2>
    └── ...
```

---

## Server API (rv-server)

Base URL: `https://<pi-tailscale-ip>:8443`

| Method | Endpoint                          | Description                        |
|--------|-----------------------------------|------------------------------------|
| GET    | `/health`                         | Server health check                |
| GET    | `/secrets`                        | List all namespaces and secrets    |
| GET    | `/secrets/{namespace}`            | List secrets in a namespace        |
| POST   | `/secrets/{namespace}/{name}`     | Push (create/update) a secret      |
| GET    | `/secrets/{namespace}/{name}`     | Pull a secret blob                 |
| DELETE | `/secrets/{namespace}/{name}`     | Delete a secret                    |

### Auth

Initially: a static bearer token set in the server config, sent as an `Authorization` header by the client. Tailscale already provides network-level trust, so this is a defense-in-depth measure.

### POST /secrets/{namespace}/{name}

**Request body:** Raw binary (the encrypted payload: `nonce || ciphertext`)

**Response:**
```json
{ "id": "uuid", "namespace": "myapp", "name": ".env", "updated_at": "..." }
```

### GET /secrets/{namespace}/{name}

**Response body:** Raw binary (the encrypted payload)

---

## CLI Commands (rv-client)

```bash
# Initialize config (server URL, auth token)
rusty_vault init

# Push a file to the vault
rusty_vault push <namespace> <filepath>
# Example: rusty_vault push myapp .env

# Pull a file from the vault (decrypt and write to disk)
rusty_vault pull <namespace> <name> [--output <path>]
# Example: rusty_vault pull myapp .env
# Example: rusty_vault pull myapp .env --output ./projects/myapp/.env

# List all secrets (optionally filter by namespace)
rusty_vault list [namespace]

# Delete a secret
rusty_vault delete <namespace> <name>

# Check server connectivity
rusty_vault ping
```

### Config File

Stored at `~/.config/rusty_vault/config.toml`

```toml
server_url = "https://100.x.x.x:8443"
auth_token = "your-static-token"
kdf_salt = "base64-encoded-random-salt"  # generated on init, stored here
```

The **passphrase is never stored** — it is prompted at runtime and used to derive the key in memory only.

---

## Tech Stack

### rv-server (Pi)

| Crate        | Purpose                        |
|--------------|--------------------------------|
| `axum`       | HTTP framework                 |
| `tokio`      | Async runtime                  |
| `rusqlite`   | SQLite for metadata            |
| `serde`      | Serialization                  |
| `tracing`    | Structured logging             |
| `uuid`       | Blob ID generation             |
| `tower-http` | Middleware (auth, CORS, etc.)  |

### rv-client (CLI)

| Crate        | Purpose                        |
|--------------|--------------------------------|
| `clap`       | CLI argument parsing           |
| `aes-gcm`    | AES-256-GCM encryption         |
| `argon2`     | Argon2id key derivation        |
| `rand`       | Nonce generation               |
| `reqwest`    | HTTP client                    |
| `serde`      | Serialization                  |
| `toml`       | Config file parsing            |
| `dirs`       | XDG config directory           |
| `rpassword`  | Secure passphrase prompt       |

---

## Project Structure (Cargo Workspace)

```
rusty_vault/
├── Cargo.toml               # Workspace root
├── README.md
├── rv-server/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Entry point, server setup
│       ├── routes/
│       │   ├── mod.rs
│       │   ├── health.rs
│       │   └── secrets.rs   # CRUD handlers
│       ├── db/
│       │   ├── mod.rs
│       │   └── schema.rs    # SQLite setup
│       ├── models.rs        # Secret struct, metadata
│       ├── auth.rs          # Bearer token middleware
│       └── config.rs        # Server config
└── rv-client/
    ├── Cargo.toml
    └── src/
        ├── main.rs          # CLI entry, clap setup
        ├── commands/
        │   ├── mod.rs
        │   ├── init.rs
        │   ├── push.rs
        │   ├── pull.rs
        │   ├── list.rs
        │   └── delete.rs
        ├── crypto.rs        # AES-GCM + Argon2 logic
        ├── client.rs        # HTTP client wrapper
        └── config.rs        # Config file read/write
```

---

## Implementation Plan

### Phase 1 — Project Scaffold & Crypto Primitives

**Goal:** Prove the crypto works in isolation before touching anything else.

Tasks:
1. Initialize Cargo workspace with two crates: `rv-server` and `rv-client`
2. Add crypto dependencies to `rv-client`
3. Implement `crypto.rs`:
   - `derive_key(passphrase: &str, salt: &[u8]) -> [u8; 32]` using Argon2id
   - `encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8>` using AES-256-GCM, returns `nonce || ciphertext`
   - `decrypt(payload: &[u8], key: &[u8; 32]) -> Result<Vec<u8>>` splits nonce, decrypts
4. Write unit tests: encrypt then decrypt, assert roundtrip equality
5. Test with a real `.env` file as input

**Exit criteria:** `cargo test` passes with a full encrypt/decrypt roundtrip on a real file.

---

### Phase 2 — CLI Skeleton & Config

**Goal:** Wire up the CLI interface and config system before adding networking.

Tasks:
1. Implement `clap` command structure in `main.rs` for all commands (push, pull, list, delete, init, ping)
2. Implement `init` command:
   - Prompt for server URL and auth token
   - Generate random KDF salt
   - Write `~/.config/rusty_vault/config.toml`
3. Implement config loading (read toml, surface useful errors)
4. Implement passphrase prompt via `rpassword` (no echo)
5. Stub out remaining commands with placeholder output

**Exit criteria:** `rusty_vault init` creates a valid config file; all commands are reachable from the CLI.

---

### Phase 3 — Server (axum on Pi)

**Goal:** A working HTTP server that stores and retrieves encrypted blobs.

Tasks:
1. Set up `axum` with `tokio` in `rv-server`
2. Implement SQLite schema via `rusqlite` (secrets table)
3. Implement server config (port, blob storage path, auth token via env or config file)
4. Implement bearer token auth middleware
5. Implement route handlers:
   - `GET /health` — returns 200 OK
   - `POST /secrets/{namespace}/{name}` — write blob to disk, upsert metadata in SQLite
   - `GET /secrets/{namespace}/{name}` — read blob from disk, stream to client
   - `GET /secrets/{namespace}` and `GET /secrets` — query SQLite, return JSON list
   - `DELETE /secrets/{namespace}/{name}` — delete blob and metadata
6. Add structured logging with `tracing`
7. Write a `systemd` service unit file for the Pi

**Exit criteria:** Server runs on Pi; can POST a file and GET it back via `curl`.

---

### Phase 4 — Client Networking & Full Commands

**Goal:** Wire up all CLI commands to the live server.

Tasks:
1. Implement `client.rs` HTTP wrapper using `reqwest`:
   - Attach auth token header
   - Handle connection errors gracefully
2. Implement `push` command:
   - Read file from disk
   - Derive key from passphrase prompt
   - Encrypt with `crypto::encrypt`
   - POST payload to server
3. Implement `pull` command:
   - GET payload from server
   - Derive key from passphrase prompt
   - Decrypt with `crypto::decrypt`
   - Write plaintext to disk (default: same filename in CWD, or `--output` path)
4. Implement `list` command: display namespaces and secrets in a clean table
5. Implement `delete` command with confirmation prompt
6. Implement `ping` command: hit `/health` and report latency

**Exit criteria:** Full push/pull roundtrip works end-to-end from laptop to Pi and back.

---

### Phase 5 — Polish & Hardening

**Goal:** Make it something you'd actually use every day, and something you'd be proud to show.

Tasks:
1. Improve error messages (no raw panics or cryptic Rust errors shown to user)
2. Handle network timeouts and server-unavailable gracefully
3. Warn if config file has insecure permissions (readable by others)
4. Add `--verbose` flag for debug output
5. Add a `rusty_vault version` command
6. Write a thorough `README.md`:
   - Setup guide (Pi server install, client install, `init` walkthrough)
   - Architecture explanation
   - Security model (what is and isn't protected)
7. Cross-compile the server binary for Pi (`armv7` or `aarch64` target)

**Exit criteria:** You can onboard a second machine in under 5 minutes using the README alone.

---

### Stretch Goal — Vaultwarden Backup Sync

**Goal:** Periodically back up encrypted blobs to Vaultwarden as an off-Pi redundancy layer.

Tasks:
1. Implement a Bitwarden-compatible API client (or use an existing Rust crate)
2. Add `rusty_vault backup` command that pushes all blobs to Vaultwarden secure notes or file attachments
3. Add `rusty_vault restore` to pull blobs back from Vaultwarden
4. Optionally schedule via cron or systemd timer on the Pi

> Since blobs are already ciphertext, Vaultwarden sees nothing sensitive — this is safe by design.

---

## Security Considerations

| Concern                  | Mitigation                                                   |
|--------------------------|--------------------------------------------------------------|
| Key exposure             | Key derived in memory only, never written to disk or config  |
| Server compromise        | Server only stores ciphertext; key is client-side only       |
| Network interception     | Tailscale provides encrypted transport (WireGuard)           |
| Unauthorized server access| Static bearer token + Tailscale network trust               |
| Passphrase brute force   | Argon2id with tuned memory/iteration parameters              |
| Nonce reuse              | Random 96-bit nonce per operation (collision risk negligible)|
| Config file exposure     | Warn if permissions are world-readable                       |

---

## Future Ideas

- Shell integration: `eval $(rusty_vault pull myapp .env --stdout)` to inject env vars into a shell session
- `--clip` flag: decrypt and copy a secret value to clipboard without writing to disk
- Secret versioning: keep previous versions of a secret on the server
- Web UI (React) on top of the server API for browsing secrets visually
- TLS with a self-signed cert on the server for an extra transport layer beyond Tailscale
