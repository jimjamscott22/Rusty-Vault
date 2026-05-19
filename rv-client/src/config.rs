//! Client configuration stored at `~/.config/rusty_vault/config.toml`.

// Config::load is not yet called — it will be used by push/pull/list/delete in Phase 4.
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Client configuration, serialized as TOML on disk.
///
/// The passphrase is **never** stored here — it is prompted at runtime and
/// used only in memory to derive the encryption key.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Base URL of the rv-server (e.g. `https://100.x.x.x:8443`).
    pub server_url: String,
    /// Static bearer token used to authenticate API requests.
    pub auth_token: String,
    /// Base64-encoded random 32-byte salt for Argon2id key derivation.
    ///
    /// Generated once on `init` and kept constant so the same passphrase
    /// always produces the same encryption key on this machine.
    pub kdf_salt: String,
}

impl Config {
    /// Returns the path to the config file: `~/.config/rusty_vault/config.toml`.
    ///
    /// # Errors
    /// Returns an error if the system config directory cannot be determined
    /// (unusual on any mainstream OS).
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("could not determine the system config directory")?;
        Ok(config_dir.join("rusty_vault").join("config.toml"))
    }

    /// Loads the config from `~/.config/rusty_vault/config.toml`.
    ///
    /// # Errors
    /// Returns an error if the file does not exist (hint: run `rusty_vault init`),
    /// cannot be read, or contains invalid TOML.
    pub fn load() -> Result<Config> {
        let path = Self::config_path()?;
        let contents = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "could not read config at {}. Run `rusty_vault init` first.",
                path.display()
            )
        })?;
        toml::from_str(&contents).context("config file contains invalid TOML")
    }

    /// Saves the config to `~/.config/rusty_vault/config.toml`.
    ///
    /// Creates the parent directory (`~/.config/rusty_vault/`) if it does not
    /// already exist.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created or the file cannot
    /// be written.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("could not create config directory at {}", parent.display())
            })?;
        }

        let contents =
            toml::to_string_pretty(self).context("could not serialize config to TOML")?;

        std::fs::write(&path, &contents)
            .with_context(|| format!("could not write config to {}", path.display()))?;

        Ok(())
    }
}
