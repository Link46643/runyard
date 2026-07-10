import { invoke } from "@tauri-apps/api/core";
import type { RunyardSettings } from "@runyard/common";
import { DEFAULT_SETTINGS } from "@runyard/common";

class SettingsStore {
  settings = $state<RunyardSettings>(DEFAULT_SETTINGS);
  loaded = $state(false);

  async load() {
    try {
      const s = await invoke<RunyardSettings>("settings_load");
      this.settings = s;
    } catch (e) {
      console.warn("[Settings] Failed to load, using defaults:", e);
      this.settings = DEFAULT_SETTINGS;
    }
    this.loaded = true;
  }

  async save() {
    try {
      await invoke("settings_save", { settings: this.settings });
    } catch (e) {
      console.error("[Settings] Failed to save:", e);
    }
  }

  async update<K extends keyof RunyardSettings>(
    section: K,
    patch: Partial<RunyardSettings[K]>
  ) {
    this.settings = {
      ...this.settings,
      [section]: {
        ...(this.settings[section] as object),
        ...patch,
      },
    };
    await this.save();
  }
}

export const settingsStore = new SettingsStore();
