//! `rusty_vault delete` — remove a secret from the vault.

use anyhow::Result;

/// Runs the `delete` command.
pub fn run(_namespace: &str, _name: &str) -> Result<()> {
    println!("delete: not yet implemented (Phase 4)");
    Ok(())
}
