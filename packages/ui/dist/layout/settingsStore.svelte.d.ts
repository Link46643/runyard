import type { RunyardSettings } from "@runyard/common";
declare class SettingsStore {
    settings: RunyardSettings;
    loaded: boolean;
    load(): Promise<void>;
    save(): Promise<void>;
    update<K extends keyof RunyardSettings>(section: K, patch: Partial<RunyardSettings[K]>): Promise<void>;
}
export declare const settingsStore: SettingsStore;
export {};
