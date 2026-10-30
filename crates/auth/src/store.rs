//! Personal access tokens in the system keychain (macOS Keychain, Secret Service on Linux),
//! service "Confluo", one item per account. Keychains cannot be enumerated, so the account
//! names (never the tokens) are also kept in an index item.

use crate::Account;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const SERVICE: &str = "Confluo";
const INDEX: &str = "confluo:index";

/// Storage of secrets by account name.
pub trait SecretBackend: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, secret: &str) -> Result<(), String>;
    fn delete(&self, account: &str) -> Result<(), String>;
}

#[derive(Clone)]
pub struct TokenStore {
    backend: Arc<dyn SecretBackend>,
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenStore")
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::keychain()
    }
}

impl TokenStore {
    /// The system keychain (an in-memory store on platforms without one).
    pub fn keychain() -> Self {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return Self::with(Arc::new(Keychain));
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Self::memory();
    }

    /// An empty in-memory store (tests, demo mode).
    pub fn memory() -> Self {
        Self::with(Arc::new(Memory::default()))
    }

    pub fn with(backend: Arc<dyn SecretBackend>) -> Self {
        Self { backend }
    }

    pub fn get(&self, account: &Account) -> Option<String> {
        self.backend
            .get(&account.name())
            .ok()
            .flatten()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
    }

    pub fn set(&self, account: &Account, token: &str) -> Result<(), String> {
        let token = token.trim();
        if token.is_empty() {
            return Err("empty token".into());
        }
        self.backend.set(&account.name(), token)?;
        let mut all = self.accounts();
        if !all.contains(account) {
            all.push(account.clone());
            self.write_index(&all)?;
        }
        Ok(())
    }

    pub fn remove(&self, account: &Account) -> Result<(), String> {
        self.backend.delete(&account.name())?;
        let all: Vec<Account> = self
            .accounts()
            .into_iter()
            .filter(|a| a != account)
            .collect();
        self.write_index(&all)
    }

    /// Accounts with a stored token, sorted.
    pub fn accounts(&self) -> Vec<Account> {
        let raw = self.backend.get(INDEX).ok().flatten().unwrap_or_default();
        let mut v: Vec<Account> = raw.lines().filter_map(Account::parse).collect();
        v.sort();
        v.dedup();
        v
    }

    fn write_index(&self, all: &[Account]) -> Result<(), String> {
        if all.is_empty() {
            return self.backend.delete(INDEX);
        }
        let names: Vec<String> = all.iter().map(Account::name).collect();
        self.backend.set(INDEX, &names.join("\n"))
    }
}

#[derive(Default)]
struct Memory(Mutex<HashMap<String, String>>);

impl SecretBackend for Memory {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(account)
            .cloned())
    }
    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        let mut m = self.0.lock().unwrap_or_else(|e| e.into_inner());
        m.insert(account.into(), secret.into());
        Ok(())
    }
    fn delete(&self, account: &str) -> Result<(), String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(account);
        Ok(())
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
struct Keychain;

#[cfg(any(target_os = "macos", target_os = "linux"))]
impl SecretBackend for Keychain {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        let e = keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())?;
        match e.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, account: &str, secret: &str) -> Result<(), String> {
        let e = keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())?;
        e.set_password(secret).map_err(|e| e.to_string())
    }
    fn delete(&self, account: &str) -> Result<(), String> {
        let e = keyring::Entry::new(SERVICE, account).map_err(|e| e.to_string())?;
        match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gv_core::ProviderKind;

    #[test]
    fn set_get_list_remove() {
        let s = TokenStore::memory();
        let a = Account::new(ProviderKind::Azure, "dev.azure.com", Some("org"));
        let g = Account::new(ProviderKind::GitHub, "github.com", None);
        assert_eq!(s.get(&a), None);
        s.set(&a, " pat \n").unwrap();
        s.set(&g, "ghp").unwrap();
        s.set(&g, "ghp2").unwrap();
        assert_eq!(s.get(&a).as_deref(), Some("pat"));
        assert_eq!(s.get(&g).as_deref(), Some("ghp2"));
        assert_eq!(s.accounts(), vec![g.clone(), a.clone()]);
        s.remove(&a).unwrap();
        assert_eq!(s.get(&a), None);
        assert_eq!(s.accounts(), vec![g]);
        assert!(s.set(&a, "  ").is_err());
    }
}
