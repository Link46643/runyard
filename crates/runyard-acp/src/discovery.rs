//! Auto-detection of ACP-compatible agents installed on the user's machine
//! (engineering-todo-v2.md 1.6.2). Two real signal sources:
//!
//! - PATH scanning for known agent executables (`claude`, `gemini`, `codex`,
//!   `goose`) - implemented here, fully unit-tested.
//! - The ACP Registry API
//!   (`https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json`) -
//!   NOT implemented in this crate, since it needs an HTTP client and this
//!   crate deliberately has none (see Cargo.toml comment on why). That half
//!   lives in `runyard-core`'s `acp_registry` module instead, hand-reviewed
//!   since `runyard-core` can't be compiled in this sandbox regardless
//!   (pre-existing GTK/glib wall) - disclosed plainly, not silently dropped.
//!
//! Manual add (executable path, spawn command, remote URL) is a pure data
//! entry operation with no discovery logic of its own - it lives directly in
//! `runyard-core`'s `acp_agent_db` CRUD commands.

use serde::{Deserialize, Serialize};

/// One well-known ACP-compatible agent Runyard actively looks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownAgent {
    pub agent_id: &'static str,
    pub display_name: &'static str,
    /// Executable names to try, in order; the first one found on PATH wins.
    pub executable_names: &'static [&'static str],
    /// Directory name to check for under the user's config home, if any
    /// (see `has_config_dir`).
    pub config_dir_name: Option<&'static str>,
}

/// The static catalog of agents Runyard scans for. Per
/// engineering-todo-v2.md 1.6.2: "scan PATH for known executables (`claude`,
/// `gemini`, `codex`, `goose`, etc.)". Cross-referenced against the live ACP
/// agents directory (agentclientprotocol.com/get-started/agents) for the
/// right executable names as of this writing.
pub const KNOWN_AGENTS: &[KnownAgent] = &[
    KnownAgent {
        agent_id: "claude-code",
        display_name: "Claude Code",
        executable_names: &["claude"],
        config_dir_name: Some("claude"),
    },
    KnownAgent {
        agent_id: "gemini-cli",
        display_name: "Gemini CLI",
        executable_names: &["gemini"],
        config_dir_name: Some("gemini"),
    },
    KnownAgent {
        agent_id: "codex-cli",
        display_name: "Codex CLI",
        executable_names: &["codex"],
        config_dir_name: Some("codex"),
    },
    KnownAgent {
        agent_id: "goose",
        display_name: "Goose",
        executable_names: &["goose"],
        config_dir_name: Some("goose"),
    },
    KnownAgent {
        agent_id: "opencode",
        display_name: "OpenCode",
        executable_names: &["opencode", "opencode-acp"],
        config_dir_name: Some("opencode"),
    },
];

/// An agent found on the local machine, ready to hand to the UI's "add
/// agent" flow as a pre-filled suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredAgent {
    pub agent_id: String,
    pub name: String,
    pub executable_path: String,
    pub has_config_dir: bool,
}

/// Scans the real `PATH` for every executable name in `KNOWN_AGENTS`. Pure
/// read-only filesystem lookups, safe to call repeatedly (e.g. on every
/// "refresh" click in the agent panel) - no caching, so it always reflects
/// what's currently installed.
pub fn discover_known_agents() -> Vec<DiscoveredAgent> {
    discover_in(KNOWN_AGENTS, None::<&str>)
}

/// Testable core of discovery: same lookup, but against an explicit catalog
/// and an explicit `PATH`-style search string (colon-separated on
/// Unix/macOS, semicolon-separated on Windows - same format as the real
/// `PATH` env var), instead of implicitly reading the process environment.
/// `search_paths: None` means "use the real PATH", matching `which::which`'s
/// own default.
pub fn discover_in(catalog: &[KnownAgent], search_paths: Option<impl AsRef<std::ffi::OsStr>>) -> Vec<DiscoveredAgent> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut found = Vec::new();
    for agent in catalog {
        let hit = agent.executable_names.iter().find_map(|exe_name| {
            which::which_in(exe_name, search_paths.as_ref(), &cwd).ok()
        });
        if let Some(path) = hit {
            found.push(DiscoveredAgent {
                agent_id: agent.agent_id.to_string(),
                name: agent.display_name.to_string(),
                executable_path: path.to_string_lossy().into_owned(),
                has_config_dir: agent.config_dir_name.map(has_config_dir).unwrap_or(false),
            });
        }
    }
    found
}

/// Best-effort check for a per-agent config directory under the user's
/// config home (`$XDG_CONFIG_HOME/<dir_name>` or `~/.config/<dir_name>` on
/// Linux/macOS, `%APPDATA%\<dir_name>` on Windows). Per 1.6.2: "parse
/// `~/.config/` directories for agent configs" - scoped to
/// existence-detection only. Each agent's config file format is
/// undocumented/proprietary, so parsing *contents* is out of scope; this
/// just gives the discovery UI an honest "this agent has run here before"
/// signal.
pub fn has_config_dir(dir_name: &str) -> bool {
    config_dir_root().map(|root| root.join(dir_name).is_dir()).unwrap_or(false)
}

fn config_dir_root() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA").map(std::path::PathBuf::from);
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let path = std::path::PathBuf::from(xdg);
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let exe_path = dir.join(name);
        std::fs::write(&exe_path, b"#!/bin/sh\necho fake\n").expect("failed to write fake executable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&exe_path, perms).unwrap();
        }
        exe_path
    }

    #[test]
    fn discovers_a_fake_agent_via_explicit_search_path() {
        let dir = std::env::temp_dir().join(format!("runyard-acp-discovery-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("failed to create scratch dir");
        let exe_name = if cfg!(target_os = "windows") { "fake-agent.exe" } else { "fake-agent" };
        write_fake_executable(&dir, exe_name);

        let catalog = &[KnownAgent {
            agent_id: "fake-agent",
            display_name: "Fake Agent",
            executable_names: &["fake-agent"],
            config_dir_name: None,
        }];
        // Explicit search path - no dependency on (or interference with)
        // the real process PATH, so this is safe under parallel test runs.
        let found = discover_in(catalog, Some(dir.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(found.len(), 1, "expected to discover exactly the fake agent, got {found:?}");
        assert_eq!(found[0].agent_id, "fake-agent");
        assert!(!found[0].executable_path.is_empty());
    }

    #[test]
    fn reports_no_agents_when_none_are_on_the_search_path() {
        let dir = std::env::temp_dir().join(format!("runyard-acp-discovery-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("failed to create scratch dir");

        let found = discover_in(KNOWN_AGENTS, Some(dir.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);
        assert!(found.is_empty(), "expected no agents on an empty search path, got {found:?}");
    }

    #[test]
    fn first_matching_executable_name_wins_without_duplicates() {
        let dir = std::env::temp_dir().join(format!("runyard-acp-discovery-dedup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("failed to create scratch dir");
        let name_a = if cfg!(target_os = "windows") { "agent-a.exe" } else { "agent-a" };
        let name_b = if cfg!(target_os = "windows") { "agent-b.exe" } else { "agent-b" };
        write_fake_executable(&dir, name_a);
        write_fake_executable(&dir, name_b);

        let catalog = &[KnownAgent {
            agent_id: "multi-name-agent",
            display_name: "Multi Name Agent",
            executable_names: &["agent-a", "agent-b"],
            config_dir_name: None,
        }];
        let found = discover_in(catalog, Some(dir.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1, "expected exactly one entry even though both alt names exist, got {found:?}");
    }

    #[test]
    fn config_dir_root_resolves_without_panicking() {
        // Exercises the real platform branch; doesn't assert about specific
        // agents having a config dir since that's machine-dependent.
        let _ = has_config_dir("claude");
    }
}
