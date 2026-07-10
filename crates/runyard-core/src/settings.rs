use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct EditorSettings {
    pub font_size: u32,
    pub tab_size: u32,
    pub line_wrap: bool,
    pub format_on_save: bool,
    pub vim_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct TerminalSettings {
    pub default_shell: Option<String>,
    pub font_size: u32,
    pub scrollback_limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub font_family: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LspLanguageConfig {
    pub enabled: bool,
    pub path_override: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct LspSettings {
    pub typescript: LspLanguageConfig,
    pub python: LspLanguageConfig,
    pub rust: LspLanguageConfig,
    pub go: LspLanguageConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct RunyardSettings {
    pub editor: EditorSettings,
    pub terminal: TerminalSettings,
    pub appearance: AppearanceSettings,
    pub lsp: LspSettings,
}

// ─── Default implementations ─────────────────────────────────────────────────

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 14,
            tab_size: 2,
            line_wrap: false,
            format_on_save: false,
            vim_mode: false,
        }
    }
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            default_shell: None,
            font_size: 13,
            scrollback_limit: 5000,
        }
    }
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "JetBrains Mono".to_string(),
        }
    }
}

impl Default for LspLanguageConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path_override: None,
        }
    }
}

impl Default for LspSettings {
    fn default() -> Self {
        Self {
            typescript: LspLanguageConfig::default(),
            python: LspLanguageConfig::default(),
            rust: LspLanguageConfig::default(),
            go: LspLanguageConfig::default(),
        }
    }
}

impl Default for RunyardSettings {
    fn default() -> Self {
        Self {
            editor: EditorSettings::default(),
            terminal: TerminalSettings::default(),
            appearance: AppearanceSettings::default(),
            lsp: LspSettings::default(),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn get_settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("runyard")
        .join("settings.json")
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn settings_load() -> Result<RunyardSettings, String> {
    let path = get_settings_path();
    if !path.exists() {
        return Ok(RunyardSettings::default());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    // Gracefully handle schema evolution — fall back to default on parse error
    serde_json::from_str::<RunyardSettings>(&content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_save(settings: RunyardSettings) -> Result<(), String> {
    let path = get_settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(())
}
