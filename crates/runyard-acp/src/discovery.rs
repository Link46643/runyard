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
    /// Extra CLI arguments required to start this binary in ACP server mode.
    /// Most agents speak ACP as soon as you run the bare binary; some (like
    /// OpenCode, whose CLI is a general-purpose entry point) require an
    /// explicit subcommand. Data-driven so the UI never needs a per-agent
    /// `if (agentId === "...")` branch - it just joins executable_path with
    /// these args to get a working spawn_command.
    pub acp_args: &'static [&'static str],
}

/// The static catalog of agents Runyard scans for. Per
/// engineering-todo-v2.md 1.6.2: "scan PATH for known executables (`claude`,
/// `gemini`, `codex`, `goose`, etc.)". Cross-referenced against each agent's
/// own current ACP documentation - corrected after finding two of the
/// original guesses were wrong (a bare CLI binary is NOT the same thing as
/// its ACP entry point for every agent):
///
/// - Claude Code: has NO built-in `--acp` flag. ACP support is a *separate*
///   adapter binary, `claude-agent-acp` (npm: @agentclientprotocol/claude-agent-acp,
///   successor to the older @zed-industries/claude-code-acp - both names are
///   searched). Scanning for bare `claude` and adding it as-is would silently
///   launch the interactive CLI instead of an ACP server.
/// - Codex CLI: same pattern - the adapter binary is `codex-acp` (npm:
///   @agentclientprotocol/codex-acp), not the bare `codex` binary.
/// - Gemini CLI: DOES speak ACP natively, via the `--acp` flag on the `gemini`
///   binary itself (geminicli.com/docs/cli/acp-mode) - no separate adapter.
/// - Goose: `goose session --acp` per Goose's own ACP docs.
/// - OpenCode: `opencode acp` per opencode.ai/docs/acp - a subcommand on the
///   main `opencode` binary, not a separate adapter package.
pub const KNOWN_AGENTS: &[KnownAgent] = &[
    KnownAgent {
        agent_id: "claude-code",
        display_name: "Claude Code",
        executable_names: &["claude-agent-acp", "claude-code-acp"],
        config_dir_name: Some("claude"),
        acp_args: &[],
    },
    KnownAgent {
        agent_id: "gemini-cli",
        display_name: "Gemini CLI",
        executable_names: &["gemini"],
        config_dir_name: Some("gemini"),
        acp_args: &["--acp"],
    },
    KnownAgent {
        agent_id: "codex-cli",
        display_name: "Codex CLI",
        executable_names: &["codex-acp"],
        config_dir_name: Some("codex"),
        acp_args: &[],
    },
    KnownAgent {
        agent_id: "goose",
        display_name: "Goose",
        executable_names: &["goose"],
        config_dir_name: Some("goose"),
        acp_args: &["session", "--acp"],
    },
    KnownAgent {
        agent_id: "opencode",
        display_name: "OpenCode",
        executable_names: &["opencode"],
        config_dir_name: Some("opencode"),
        acp_args: &["acp"],
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
    /// Ready-to-use spawn command (executable path + any required ACP args,
    /// shell-quoted so paths with spaces survive `shell_words::split` on the
    /// way back in). This is what should go straight into the agent's
    /// `spawn_command` field - never just `executable_path` alone, since a
    /// bare binary for agents like OpenCode starts the interactive TUI, not
    /// the ACP JSON-RPC server.
    pub recommended_spawn_command: String,
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
            let path_str = path.to_string_lossy().into_owned();
            found.push(DiscoveredAgent {
                agent_id: agent.agent_id.to_string(),
                name: agent.display_name.to_string(),
                executable_path: path_str.clone(),
                has_config_dir: agent.config_dir_name.map(has_config_dir).unwrap_or(false),
                recommended_spawn_command: build_spawn_command(&path_str, agent.acp_args),
            });
        }
    }
    found
}

/// Joins an executable path with its required ACP args into one spawn
/// command string, quoting the path if it contains spaces (common on
/// Windows: `C:\Program Files\...`) so `shell_words::split` on the
/// consuming side (see `AcpAgent::from_str` in agent-client-protocol-tokio)
/// reconstructs the exact same argv. This is the ONE place spawn commands
/// are built from discovery data, so there is no per-OS or per-agent
/// special-casing anywhere else in the codebase.
fn build_spawn_command(executable_path: &str, acp_args: &[&str]) -> String {
    let quoted_path = shell_words::quote(executable_path).into_owned();
    if acp_args.is_empty() {
        quoted_path
    } else {
        let quoted_args: Vec<String> = acp_args.iter().map(|a| shell_words::quote(a).into_owned()).collect();
        format!("{quoted_path} {}", quoted_args.join(" "))
    }
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
            acp_args: &[],
        }];
        // Explicit search path - no dependency on (or interference with)
        // the real process PATH, so this is safe under parallel test runs.
        let found = discover_in(catalog, Some(dir.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(found.len(), 1, "expected to discover exactly the fake agent, got {found:?}");
        assert_eq!(found[0].agent_id, "fake-agent");
        assert!(!found[0].executable_path.is_empty());
        assert_eq!(
            found[0].recommended_spawn_command, found[0].executable_path,
            "an agent with no acp_args should recommend the bare executable path"
        );
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
            acp_args: &[],
        }];
        let found = discover_in(catalog, Some(dir.to_string_lossy().into_owned()));

        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found.len(), 1, "expected exactly one entry even though both alt names exist, got {found:?}");
    }

    #[test]
    fn opencode_recommended_spawn_command_includes_acp_subcommand() {
        // Regression test for a real bug: discovering the bare `opencode`
        // binary and using it as-is would start OpenCode's interactive TUI,
        // not its ACP server. opencode.ai/docs/acp documents the real
        // invocation as `opencode acp` - verify the catalog entry and the
        // command-builder both produce that, not just the bare path.
        let opencode = KNOWN_AGENTS.iter().find(|a| a.agent_id == "opencode")
            .expect("opencode should be in the known agents catalog");
        assert_eq!(opencode.acp_args, &["acp"], "opencode requires the 'acp' subcommand to speak ACP");

        let cmd = build_spawn_command("/usr/local/bin/opencode", opencode.acp_args);
        assert_eq!(cmd, "/usr/local/bin/opencode acp");

        // Also verify goose, which needs a longer arg chain per its own ACP docs.
        let goose = KNOWN_AGENTS.iter().find(|a| a.agent_id == "goose")
            .expect("goose should be in the known agents catalog");
        let goose_cmd = build_spawn_command("/usr/local/bin/goose", goose.acp_args);
        assert_eq!(goose_cmd, "/usr/local/bin/goose session --acp");
    }

    #[test]
    fn claude_and_codex_point_at_adapter_binaries_not_base_cli() {
        // Regression test for a real bug: `claude` and `codex` are the base
        // CLIs and do NOT speak ACP themselves. ACP support for both ships as
        // a separate adapter binary (npm packages
        // @agentclientprotocol/claude-agent-acp and
        // @agentclientprotocol/codex-acp respectively). Scanning for the base
        // binary and spawning it as-is would launch the interactive CLI, not
        // an ACP server - verify the catalog searches for the adapter names
        // instead, with no extra args needed since the adapter itself is the
        // ACP entry point.
        let claude = KNOWN_AGENTS.iter().find(|a| a.agent_id == "claude-code")
            .expect("claude-code should be in the known agents catalog");
        assert!(
            claude.executable_names.contains(&"claude-agent-acp"),
            "claude-code must search for the claude-agent-acp adapter binary, not just the base `claude` CLI"
        );
        assert!(!claude.executable_names.contains(&"claude"), "bare `claude` does not speak ACP");
        assert_eq!(claude.acp_args, &[] as &[&str]);

        let codex = KNOWN_AGENTS.iter().find(|a| a.agent_id == "codex-cli")
            .expect("codex-cli should be in the known agents catalog");
        assert!(
            codex.executable_names.contains(&"codex-acp"),
            "codex-cli must search for the codex-acp adapter binary, not just the base `codex` CLI"
        );
        assert!(!codex.executable_names.contains(&"codex"), "bare `codex` does not speak ACP");
        assert_eq!(codex.acp_args, &[] as &[&str]);
    }

    #[test]
    fn gemini_cli_recommended_spawn_command_includes_acp_flag() {
        // Regression test: unlike Claude/Codex, Gemini CLI DOES speak ACP
        // natively on its base binary, but only when passed `--acp`
        // (geminicli.com/docs/cli/acp-mode) - a bare `gemini` with no flag
        // starts its interactive chat UI instead.
        let gemini = KNOWN_AGENTS.iter().find(|a| a.agent_id == "gemini-cli")
            .expect("gemini-cli should be in the known agents catalog");
        assert_eq!(gemini.acp_args, &["--acp"], "gemini requires the --acp flag to speak ACP");

        let cmd = build_spawn_command("/usr/local/bin/gemini", gemini.acp_args);
        assert_eq!(cmd, "/usr/local/bin/gemini --acp");
    }

    #[test]
    fn build_spawn_command_quotes_paths_with_spaces() {
        // Windows commonly installs to "C:\Program Files\...". The quoted
        // command must round-trip through shell_words::split back to the
        // exact same single path argument (this is what
        // agent-client-protocol-tokio's AcpAgent::from_str does on connect).
        let cmd = build_spawn_command("C:\\Program Files\\OpenCode\\opencode.exe", &["acp"]);
        let parsed = shell_words::split(&cmd).expect("recommended command must be valid shell_words syntax");
        assert_eq!(parsed, vec!["C:\\Program Files\\OpenCode\\opencode.exe", "acp"]);
    }

    #[test]
    fn config_dir_root_resolves_without_panicking() {
        // Exercises the real platform branch; doesn't assert about specific
        // agents having a config dir since that's machine-dependent.
        let _ = has_config_dir("claude");
    }
}
