//! `rusty_vault list` — list secrets stored in the vault.

use anyhow::Result;

/// Runs the `list` command.
pub fn run(_namespace: Option<&str>) -> Result<()> {
    println!("list: not yet implemented (Phase 4)");
    Ok(())
}
