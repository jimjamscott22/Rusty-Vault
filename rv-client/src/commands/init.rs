//! `rusty_vault init` — creates the client configuration file interactively.

use crate::config::Config;
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use rand::RngCore;
use std::io::{self, Write};

/// Prompts for server URL and auth token, generates a KDF salt, and writes
/// the config to `~/.config/rusty_vault/config.toml`.
pub fn run() -> Result<()> {
    println!("Initializing Rusty Vault...");
    println!();

    let server_url = prompt("Server URL (e.g. https://100.x.x.x:8443): ")?;
    let auth_token = rpassword::prompt_password("Auth token: ")?;

    // Generate a cryptographically random 32-byte KDF salt. This is stored in
    // the config so the same passphrase always derives the same key on this
    // machine. It is NOT a secret — it just prevents precomputed attacks.
    let mut salt_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt_bytes);
    let kdf_salt = general_purpose::STANDARD.encode(salt_bytes);

    let config = Config {
        server_url,
        auth_token,
        kdf_salt,
    };

    config.save()?;

    let path = Config::config_path()?;
    println!();
    println!("Configuration saved to: {}", path.display());
    println!("A unique KDF salt has been generated and stored in the config.");
    println!("Your passphrase is never stored — you will be prompted at runtime.");
    println!();
    println!("Next steps:");
    println!("  rusty_vault ping           # verify server connectivity");
    println!("  rusty_vault push <ns> .env # push your first secret");

    Ok(())
}

/// Prints a prompt and reads a line from stdin.
fn prompt(message: &str) -> Result<String> {
    print!("{}", message);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
