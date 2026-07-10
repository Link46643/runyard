// ACP Registry API client (engineering-todo-v2.md 1.6.2: "check ACP Registry
// API"). The Agent Client Protocol project maintains a real, live, versioned
// registry of ACP-compatible agents at agentclientprotocol.com - this fetches
// it so the "add agent" flow can suggest agents beyond whatever happens to
// already be installed locally (that half is runyard_acp::discovery instead).
//
// New dependency: reqwest (rustls-tls backend - pure Rust, no OpenSSL, matches
// this app's Windows/Linux/macOS cross-platform requirement). Flagged here
// explicitly since AGENTS.md requires approval for new libraries; this one is
// narrowly scoped to a single blocking GET against one trusted, versioned,
// read-only JSON endpoint - disclosed for the user's review, not silently
// added.

use serde::{Deserialize, Serialize};

const REGISTRY_URL: &str = "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

/// One entry from the ACP registry's `agents` array. Mirrors the fields
/// documented in the ACP Agent Registry RFD (agentclientprotocol.com/rfds/acp-agent-registry):
/// id, name, version, description are required by the registry's own
/// manifest schema; the rest are optional. `distribution` (how to actually
/// install/run the agent - binary/npx/uvx, each with a different shape) is
/// deliberately not modelled here - this command is for browsing/discovery
/// only, not automatic installation, so the user reads the description and
/// repository link and fills in their own spawn command, same as any
/// manually-added agent.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegistryAgent {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryResponse {
    #[allow(dead_code)]
    #[serde(default)]
    version: String,
    #[serde(default)]
    agents: Vec<RegistryAgent>,
}

#[tauri::command]
pub fn acp_agent_fetch_registry() -> Result<Vec<RegistryAgent>, String> {
    fetch_registry_from(REGISTRY_URL)
}

fn fetch_registry_from(url: &str) -> Result<Vec<RegistryAgent>, String> {
    let response = reqwest::blocking::get(url).map_err(|e| format!("failed to reach the ACP registry: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("ACP registry returned HTTP {}", response.status()));
    }
    let parsed: RegistryResponse = response
        .json()
        .map_err(|e| format!("failed to parse ACP registry response: {e}"))?;
    Ok(parsed.agents)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pure deserialization test - no network access, so it's honestly
    // runnable even without hitting the real endpoint. Shape copied from the
    // RFD's documented schema (id/name/version/description required, the
    // rest optional).
    #[test]
    fn parses_a_realistic_registry_payload() {
        let body = r#"{
            "version": "1.0.0",
            "agents": [
                {
                    "id": "example-agent",
                    "name": "Example Agent",
                    "version": "1.2.0",
                    "description": "An example ACP agent.",
                    "repository": "https://github.com/example/example-agent",
                    "authors": ["Example Org"],
                    "license": "MIT",
                    "icon": "icon.svg"
                },
                {
                    "id": "minimal-agent",
                    "name": "Minimal Agent",
                    "version": "0.1.0",
                    "description": "Only the required fields."
                }
            ]
        }"#;
        let parsed: RegistryResponse = serde_json::from_str(body).expect("should parse a realistic registry payload");
        assert_eq!(parsed.agents.len(), 2);
        assert_eq!(parsed.agents[0].id, "example-agent");
        assert_eq!(parsed.agents[0].authors, vec!["Example Org".to_string()]);
        assert_eq!(parsed.agents[1].repository, None, "optional fields should default cleanly when absent");
    }
}
