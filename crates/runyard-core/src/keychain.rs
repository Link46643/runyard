// OS keychain integration for secret agent env vars (engineering-todo-v2.md
// 1.6.8). Uses the `keyring` crate which maps to:
//   macOS  → Keychain Services
//   Windows → Credential Manager
//   Linux  → libsecret (GNOME Keyring) or kwallet (KDE)
//
// Service name strategy: one entry per (agent_id, key) pair under the
// "runyard-acp" service namespace, so secrets are scoped to the agent that
// owns them and never bleed across agents with the same key name.

const SERVICE: &str = "runyard-acp";

/// Build the keyring account name for a given agent + key pair.
fn account_name(agent_id: &str, key: &str) -> String {
    format!("{agent_id}:{key}")
}

/// Store a secret value in the OS keychain.
/// Replaces any existing entry for the same agent_id + key.
#[tauri::command]
pub fn keychain_set(agent_id: String, key: String, value: String) -> Result<(), String> {
    let account = account_name(&agent_id, &key);
    let entry = keyring::Entry::new(SERVICE, &account)
        .map_err(|e| format!("keyring entry creation failed: {e}"))?;
    entry.set_password(&value)
        .map_err(|e| format!("keychain set failed for {account}: {e}"))
}

/// Retrieve a secret value from the OS keychain.
/// Returns `None` if no entry exists for this agent + key (not an error).
#[tauri::command]
pub fn keychain_get(agent_id: String, key: String) -> Result<Option<String>, String> {
    let account = account_name(&agent_id, &key);
    let entry = keyring::Entry::new(SERVICE, &account)
        .map_err(|e| format!("keyring entry creation failed: {e}"))?;
    match entry.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keychain get failed for {account}: {e}")),
    }
}

/// Delete a secret from the OS keychain.
/// A missing entry is treated as success (idempotent delete).
#[tauri::command]
pub fn keychain_delete(agent_id: String, key: String) -> Result<(), String> {
    let account = account_name(&agent_id, &key);
    let entry = keyring::Entry::new(SERVICE, &account)
        .map_err(|e| format!("keyring entry creation failed: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // already gone
        Err(e) => Err(format!("keychain delete failed for {account}: {e}")),
    }
}

/// Bulk-retrieve all secret env var values for an agent, combining the
/// cleartext env_vars from the DB (non-secret entries) with the secret values
/// from the OS keychain. The returned map is keyed by env var name and contains
/// the real value for every entry - suitable for passing to a subprocess as its
/// environment.
#[tauri::command]
pub fn keychain_resolve_agent_env(
    agent_id: String,
    env_vars: Vec<crate::acp_agent_db::DbAcpAgentEnvVar>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let mut resolved = std::collections::HashMap::new();
    for var in env_vars {
        if var.is_secret {
            // Try to load from keychain; fall back to the stored value if the
            // keychain doesn't have an entry yet (e.g. during initial import).
            match keychain_get(agent_id.clone(), var.key.clone())? {
                Some(secret_value) => { resolved.insert(var.key, secret_value); }
                None if !var.value.is_empty() => { resolved.insert(var.key, var.value); }
                None => {} // No value available; skip rather than set empty.
            }
        } else {
            resolved.insert(var.key, var.value);
        }
    }
    Ok(resolved)
}
