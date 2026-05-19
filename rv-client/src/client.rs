//! HTTP client wrapper for communicating with rv-server.
//!
//! Not yet implemented — networking will be wired up in Phase 4.

use reqwest::blocking::Client;

/// Wraps a `reqwest` blocking HTTP client pre-configured with the server
/// base URL and bearer token.
#[allow(dead_code)]
pub struct VaultClient {
    pub base_url: String,
    auth_token: String,
    client: Client,
}

impl VaultClient {
    /// Creates a new client ready to talk to the given server.
    #[allow(dead_code)]
    pub fn new(base_url: String, auth_token: String) -> Self {
        Self {
            base_url,
            auth_token,
            client: Client::new(),
        }
    }
}
