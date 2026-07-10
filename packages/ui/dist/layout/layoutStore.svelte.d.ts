import type { Layout, Tab } from "@runyard/common";
declare class LayoutStore {
    layout: Layout;
    /** LIFO stack of recently closed non-terminal tabs (max 10). */
    closedTabHistory: Tab[];
    constructor();
    load(): void;
    save(): void;
    private findNode;
    private findParentOf;
    addTab(leafId: string, tab: Tab): void;
    closeTab(tabId: string, force?: boolean): boolean;
    /** Re-open the most recently closed non-terminal tab. */
    reopenLastTab(): void;
    /**
     * Update a tab's display title in-place (without persisting to localStorage).
     * Used for ephemeral updates like terminal shell name changes.
     */
    setTabTitle(tabId: string, title: string): void;
    setActiveTab(tabId: string): void;
    moveTab(tabId: string, targetLeafId: string): void;
    splitLeaf(leafId: string, direction: "horizontal" | "vertical"): void;
    resizeLeaf(splitId: string, sizes: number[]): void;
    popOutTab(tabId: string): void;
    setTabDirty(tabId: string, dirty: boolean): void;
    private findFirstLeafNotExplorer;
    private findFirstLeaf;
    openEditor(path: string, name: string): void;
    clearLayout(): void;
    /** Open a new terminal tab. Invokes terminal_create on the Rust backend. */
    openTerminal(cwd?: string): Promise<void>;
    /** Open (or focus) the Git panel. */
    openGit(workspacePath?: string): void;
    /** Open (or focus) the Settings panel. */
    openSettings(): void;
}
export declare const layoutEngine: LayoutStore;
export {};
