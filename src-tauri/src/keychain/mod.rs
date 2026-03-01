//! OS Keychain integration for secure API key storage.
//!
//! This module provides secure storage of API keys using the operating system's
//! keychain/keyring functionality.

use keyring::Entry;
use thiserror::Error;

/// Service name for keychain entries.
const SERVICE_NAME: &str = "coder-ai";

/// Errors from keychain operations.
#[derive(Debug, Error)]
pub enum KeychainError {
    /// Keychain is unavailable
    #[error("Keychain unavailable: {0}")]
    Unavailable(String),
    /// Access denied
    #[error("Access denied: {0}")]
    AccessDenied(String),
    /// Entry not found
    #[error("Entry not found: {0}")]
    NotFound(String),
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Stores an API key in the OS keychain.
///
/// # Errors
///
/// Returns an error if the keychain is unavailable or the operation fails.
pub fn store_api_key(provider_id: &str, api_key: &str) -> Result<(), KeychainError> {
    let entry = Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| KeychainError::Unavailable(e.to_string()))?;

    entry
        .set_password(api_key)
        .map_err(|e| KeychainError::Storage(e.to_string()))?;

    Ok(())
}

/// Retrieves an API key from the OS keychain.
///
/// # Errors
///
/// Returns an error if the keychain is unavailable or the entry is not found.
pub fn retrieve_api_key(provider_id: &str) -> Result<String, KeychainError> {
    let entry = Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| KeychainError::Unavailable(e.to_string()))?;

    entry.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => KeychainError::NotFound(provider_id.to_string()),
        _ => KeychainError::AccessDenied(e.to_string()),
    })
}

/// Deletes an API key from the OS keychain.
///
/// # Errors
///
/// Returns an error if the keychain is unavailable or the entry is not found.
pub fn delete_api_key(provider_id: &str) -> Result<(), KeychainError> {
    let entry = Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| KeychainError::Unavailable(e.to_string()))?;

    entry.delete_credential().map_err(|e| match e {
        keyring::Error::NoEntry => KeychainError::NotFound(provider_id.to_string()),
        _ => KeychainError::Storage(e.to_string()),
    })
}

/// Checks if an API key exists in the OS keychain.
///
/// # Errors
///
/// Returns an error if the keychain is unavailable.
pub fn has_api_key(provider_id: &str) -> Result<bool, KeychainError> {
    let entry = Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| KeychainError::Unavailable(e.to_string()))?;

    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(KeychainError::AccessDenied(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests interact with the actual OS keychain.
    // They may require user interaction on some systems.

    #[test]
    fn test_store_and_retrieve() {
        let test_id = "test-provider-store";
        let test_key = "test-api-key-12345";

        // Store
        let result = store_api_key(test_id, test_key);
        if result.is_err() {
            // Keychain may not be available in CI
            return;
        }

        // Retrieve
        let retrieved = retrieve_api_key(test_id).expect("Failed to retrieve key");
        assert_eq!(retrieved, test_key);

        // Cleanup
        let _ = delete_api_key(test_id);
    }

    #[test]
    fn test_retrieve_nonexistent() {
        let result = retrieve_api_key("nonexistent-provider-xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_api_key() {
        let test_id = "test-provider-has";

        // Should not exist initially
        if let Ok(exists) = has_api_key(test_id) {
            assert!(!exists);
        }

        // Store
        if store_api_key(test_id, "test-key").is_ok() {
            // Should exist now
            let exists = has_api_key(test_id).expect("Failed to check key");
            assert!(exists);

            // Cleanup
            let _ = delete_api_key(test_id);
        }
    }
}
