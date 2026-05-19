use anyhow::Result;
use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;
mod crypto;

#[derive(Parser)]
#[command(
    name = "rusty_vault",
    about = "A self-hosted CLI secrets vault backed by a Raspberry Pi over Tailscale"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize the vault (server URL, auth token, KDF salt)
    Init,

    /// Encrypt and push a file to the vault
    Push {
        /// Namespace for organizing secrets (e.g. `myapp`, `homelab`)
        namespace: String,
        /// Path to the file to encrypt and upload
        filepath: String,
    },

    /// Fetch and decrypt a file from the vault
    Pull {
        /// Namespace the secret belongs to
        namespace: String,
        /// Name of the secret (e.g. `.env`)
        name: String,
        /// Write the decrypted file to this path (defaults to `./<name>`)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// List secrets in the vault
    List {
        /// Filter by namespace (lists all namespaces if omitted)
        namespace: Option<String>,
    },

    /// Delete a secret from the vault
    Delete {
        /// Namespace the secret belongs to
        namespace: String,
        /// Name of the secret to delete
        name: String,
    },

    /// Check connectivity to the vault server
    Ping,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => commands::init::run(),
        Command::Push { namespace, filepath } => commands::push::run(&namespace, &filepath),
        Command::Pull {
            namespace,
            name,
            output,
        } => commands::pull::run(&namespace, &name, output.as_deref()),
        Command::List { namespace } => commands::list::run(namespace.as_deref()),
        Command::Delete { namespace, name } => commands::delete::run(&namespace, &name),
        Command::Ping => commands::ping::run(),
    }
}
