//! Secure credential storage via OS keyring.
//!
//! Provides cross-platform access to:
//! - macOS Keychain
//! - Windows Credential Manager
//! - Linux Secret Service (GNOME Keyring, KWallet)

use anyhow::{Context, Result};
use tracing::{debug, warn};

const SERVICE_NAME: &str = "bourso-cli";

/// Try to get password from keyring.
/// Returns None if unavailable or not found.
pub fn try_get_password(customer_id: &str) -> Option<String> {
    match keyring::Entry::new(SERVICE_NAME, customer_id) {
        Ok(entry) => match entry.get_password() {
            Ok(password) => {
                debug!("Password retrieved from OS keyring");
                Some(password)
            }
            Err(keyring::Error::NoEntry) => {
                debug!("No password found in keyring for customer {}", customer_id);
                None
            }
            Err(e) => {
                warn!("Failed to access keyring: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Keyring unavailable: {}", e);
            None
        }
    }
}

/// Store password in the OS keyring.
pub fn store_password(customer_id: &str, password: &str) -> Result<()> {
    let entry =
        keyring::Entry::new(SERVICE_NAME, customer_id).context("Failed to access keyring")?;
    entry
        .set_password(password)
        .context("Failed to store password in keyring")?;
    Ok(())
}

/// Delete password from the OS keyring.
pub fn delete_password(customer_id: &str) -> Result<()> {
    let entry =
        keyring::Entry::new(SERVICE_NAME, customer_id).context("Failed to access keyring")?;
    entry
        .delete_credential()
        .context("Failed to delete password from keyring")?;
    Ok(())
}

/// Check if keyring is available on this system.
pub fn is_available() -> bool {
    keyring::Entry::new(SERVICE_NAME, "__test__").is_ok()
}
