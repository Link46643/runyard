import { appStatus } from "./appStatusStore.svelte.js";
import { settingsStore } from "./settingsStore.svelte.js";
import { invoke } from "@tauri-apps/api/core";
const DEFAULT_LAYOUT = {
    root: {
        type: "leaf",
        id: "root-leaf",
        tabs: [],
        activeTabId: null
    }
};
class LayoutStore {
    layout = $state(DEFAULT_LAYOUT);
    /** LIFO stack of recently closed non-terminal tabs (max 10). */
    closedTabHistory = $state([]);
    constructor() {
        this.load();
    }
    load() {
        if (typeof window !== "undefined") {
            const saved = localStorage.getItem("runyard:layout");
            if (saved) {
                try {
                    this.layout = JSON.parse(saved);
                }
                catch (e) {
                    console.error("Failed to parse layout from localStorage", e);
                }
            }
        }
    }
    save() {
        if (typeof window !== "undefined") {
            localStorage.setItem("runyard:layout", JSON.stringify(this.layout));
        }
    }
    findNode(node, id) {
        if (node.id === id)
            return node;
        if (node.type === "split") {
            for (const child of node.children) {
                const found = this.findNode(child, id);
                if (found)
                    return found;
            }
        }
        return null;
    }
    findParentOf(node, targetId) {
        if (node.type === "leaf")
            return null;
        if (node.children.some(c => c.id === targetId))
            return node;
        for (const child of node.children) {
            const parent = this.findParentOf(child, targetId);
            if (parent)
                return parent;
        }
        return null;
    }
    addTab(leafId, tab) {
        console.log(`[layoutStore] addTab: adding ${tab.title} (${tab.type}) to leaf ${leafId}`);
        const leaf = this.findNode(this.layout.root, leafId);
        if (leaf && leaf.type === "leaf") {
            leaf.tabs.push(tab);
            leaf.activeTabId = tab.id;
            this.save();
        }
        else {
            console.warn(`[layoutStore] addTab: could not find leaf ${leafId}`);
        }
    }
    closeTab(tabId, force = false) {
        console.log(`[layoutStore] closeTab: closing ${tabId} (force: ${force})`);
        // Check if dirty and not forced
        const findTab = (node) => {
            if (node.type === "leaf") {
                return node.tabs.find(t => t.id === tabId) || null;
            }
            if (node.type === "split") {
                for (const child of node.children) {
                    const found = findTab(child);
                    if (found)
                        return found;
                }
            }
            return null;
        };
        const tab = findTab(this.layout.root);
        if (tab && tab.dirty && !force) {
            return false;
        }
        const removeTabFromNode = (node) => {
            if (node.type === "leaf") {
                const idx = node.tabs.findIndex(t => t.id === tabId);
                if (idx !== -1) {
                    const newTabs = [...node.tabs];
                    newTabs.splice(idx, 1);
                    node.tabs = newTabs; // Force trigger deep reactivity
                    if (node.activeTabId === tabId) {
                        node.activeTabId = node.tabs.length > 0 ? node.tabs[Math.max(0, idx - 1)].id : null;
                    }
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (removeTabFromNode(child))
                        return true;
                }
            }
            return false;
        };
        if (removeTabFromNode(this.layout.root)) {
            // Track non-terminal tabs so they can be reopened (terminal PTY sessions are dead after close)
            if (tab && tab.type !== "terminal") {
                this.closedTabHistory = [...this.closedTabHistory, { ...tab }].slice(-10);
            }
            this.save();
        }
        return true;
    }
    /** Re-open the most recently closed non-terminal tab. */
    reopenLastTab() {
        if (this.closedTabHistory.length === 0)
            return;
        const tab = this.closedTabHistory[this.closedTabHistory.length - 1];
        this.closedTabHistory = this.closedTabHistory.slice(0, -1);
        // If the tab is already open somewhere, just focus it
        const alreadyOpen = (node) => {
            if (node.type === "leaf")
                return node.tabs.some(t => t.id === tab.id);
            if (node.type === "split")
                return node.children.some(c => alreadyOpen(c));
            return false;
        };
        if (alreadyOpen(this.layout.root)) {
            this.setActiveTab(tab.id);
            return;
        }
        const targetLeaf = this.findFirstLeafNotExplorer(this.layout.root) ||
            this.findFirstLeaf(this.layout.root);
        if (targetLeaf) {
            targetLeaf.tabs.push(tab);
            targetLeaf.activeTabId = tab.id;
            this.save();
        }
    }
    /**
     * Update a tab's display title in-place (without persisting to localStorage).
     * Used for ephemeral updates like terminal shell name changes.
     */
    setTabTitle(tabId, title) {
        const updateInNode = (node) => {
            if (node.type === "leaf") {
                const tab = node.tabs.find(t => t.id === tabId);
                if (tab) {
                    tab.title = title;
                    return;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children)
                    updateInNode(child);
            }
        };
        updateInNode(this.layout.root);
        // Intentionally NOT calling this.save() — tab titles are ephemeral;
        // they reset to "Terminal" on reload, which is expected.
    }
    setActiveTab(tabId) {
        const setInNode = (node) => {
            if (node.type === "leaf") {
                if (node.tabs.some(t => t.id === tabId)) {
                    node.activeTabId = tabId;
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (setInNode(child))
                        return true;
                }
            }
            return false;
        };
        if (setInNode(this.layout.root)) {
            this.save();
        }
    }
    moveTab(tabId, targetLeafId) {
        let foundTab = null;
        const removeTabFromNode = (node) => {
            if (node.type === "leaf") {
                const idx = node.tabs.findIndex(t => t.id === tabId);
                if (idx !== -1) {
                    foundTab = node.tabs[idx];
                    node.tabs.splice(idx, 1);
                    if (node.activeTabId === tabId) {
                        node.activeTabId = node.tabs.length > 0 ? node.tabs[Math.max(0, idx - 1)].id : null;
                    }
                }
            }
            else if (node.type === "split") {
                for (const child of node.children)
                    removeTabFromNode(child);
            }
        };
        removeTabFromNode(this.layout.root);
        if (foundTab) {
            // TypeScript narrows closure-captured variables pessimistically; cast to Tab is safe
            // because we are inside the if (foundTab) truthy guard.
            const tab = foundTab;
            const targetLeaf = this.findNode(this.layout.root, targetLeafId);
            if (targetLeaf && targetLeaf.type === "leaf") {
                targetLeaf.tabs.push(tab);
                targetLeaf.activeTabId = tab.id;
                this.save();
            }
        }
    }
    splitLeaf(leafId, direction) {
        if (this.layout.root.id === leafId && this.layout.root.type === "leaf") {
            const oldRoot = this.layout.root;
            const newLeaf1 = { ...oldRoot, id: crypto.randomUUID() };
            const newLeaf2 = { type: "leaf", id: crypto.randomUUID(), tabs: [], activeTabId: null };
            this.layout.root = {
                type: "split",
                id: crypto.randomUUID(),
                direction,
                children: [newLeaf1, newLeaf2],
                sizes: [50, 50]
            };
            this.save();
            return;
        }
        const parent = this.findParentOf(this.layout.root, leafId);
        if (parent) {
            const idx = parent.children.findIndex(c => c.id === leafId);
            if (idx !== -1) {
                const oldLeaf = parent.children[idx];
                const newLeaf1 = { ...oldLeaf, id: crypto.randomUUID() };
                const newLeaf2 = { type: "leaf", id: crypto.randomUUID(), tabs: [], activeTabId: null };
                const newSplit = {
                    type: "split",
                    id: crypto.randomUUID(),
                    direction,
                    children: [newLeaf1, newLeaf2],
                    sizes: [50, 50]
                };
                parent.children.splice(idx, 1, newSplit);
                this.save();
            }
        }
    }
    resizeLeaf(splitId, sizes) {
        const split = this.findNode(this.layout.root, splitId);
        if (split && split.type === "split") {
            split.sizes = sizes;
            this.save();
        }
    }
    popOutTab(tabId) {
        console.log(`[popOutTab] Popping out tab ${tabId} into a new window...`);
    }
    setTabDirty(tabId, dirty) {
        const updateInNode = (node) => {
            if (node.type === "leaf") {
                const tab = node.tabs.find(t => t.id === tabId);
                if (tab && tab.dirty !== dirty) {
                    tab.dirty = dirty;
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (updateInNode(child))
                        return true;
                }
            }
            return false;
        };
        if (updateInNode(this.layout.root)) {
            this.save();
        }
    }
    findFirstLeafNotExplorer(node) {
        if (node.type === "leaf") {
            if (!node.tabs.some(t => t.type === "explorer"))
                return node;
            return null;
        }
        if (node.type === "split") {
            for (const child of node.children) {
                const found = this.findFirstLeafNotExplorer(child);
                if (found)
                    return found;
            }
        }
        return null;
    }
    findFirstLeaf(node) {
        if (node.type === "leaf")
            return node;
        if (node.type === "split") {
            for (const child of node.children) {
                const found = this.findFirstLeaf(child);
                if (found)
                    return found;
            }
        }
        return null;
    }
    openEditor(path, name) {
        const tabId = path;
        const setFocus = (node) => {
            if (node.type === "leaf") {
                if (node.tabs.some(t => t.id === tabId)) {
                    node.activeTabId = tabId;
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (setFocus(child))
                        return true;
                }
            }
            return false;
        };
        if (setFocus(this.layout.root)) {
            this.save();
            return;
        }
        const newTab = {
            id: tabId,
            type: "editor",
            title: name,
            props: { filePath: path }
        };
        let targetLeaf = this.findFirstLeafNotExplorer(this.layout.root) || this.findFirstLeaf(this.layout.root);
        if (targetLeaf) {
            appStatus.markAsJustOpened(path);
            targetLeaf.tabs.push(newTab);
            targetLeaf.activeTabId = newTab.id;
            this.save();
        }
    }
    clearLayout() {
        this.layout = {
            root: {
                type: "leaf",
                id: "root-leaf",
                tabs: [],
                activeTabId: null
            }
        };
        this.save();
    }
    /** Open a new terminal tab. Invokes terminal_create on the Rust backend. */
    async openTerminal(cwd) {
        // Forward the user's configured default shell (null = use OS $SHELL/$COMSPEC)
        const shell = settingsStore.settings.terminal.default_shell || null;
        let info;
        try {
            info = await invoke("terminal_create", { cwd: cwd ?? null, shell });
        }
        catch (e) {
            console.error("[layoutStore] Failed to create terminal", e);
            return;
        }
        const tab = {
            id: `terminal:${info.id}`,
            type: "terminal",
            title: "Terminal",
            props: { terminalId: info.id, cwd: info.cwd },
        };
        const targetLeaf = this.findFirstLeafNotExplorer(this.layout.root) ||
            this.findFirstLeaf(this.layout.root);
        if (targetLeaf) {
            targetLeaf.tabs.push(tab);
            targetLeaf.activeTabId = tab.id;
            this.save();
        }
    }
    /** Open (or focus) the Git panel. */
    openGit(workspacePath = "../../") {
        const tabId = `git:${workspacePath}`;
        const setFocus = (node) => {
            if (node.type === "leaf") {
                if (node.tabs.some((t) => t.id === tabId)) {
                    node.activeTabId = tabId;
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (setFocus(child))
                        return true;
                }
            }
            return false;
        };
        if (setFocus(this.layout.root)) {
            this.save();
            return;
        }
        const tab = {
            id: tabId,
            type: "git",
            title: "Git",
            props: { workspacePath },
        };
        const targetLeaf = this.findFirstLeafNotExplorer(this.layout.root) ||
            this.findFirstLeaf(this.layout.root);
        if (targetLeaf) {
            targetLeaf.tabs.push(tab);
            targetLeaf.activeTabId = tab.id;
            this.save();
        }
    }
    /** Open (or focus) the Settings panel. */
    openSettings() {
        const tabId = "settings";
        const setFocus = (node) => {
            if (node.type === "leaf") {
                if (node.tabs.some((t) => t.id === tabId)) {
                    node.activeTabId = tabId;
                    return true;
                }
            }
            else if (node.type === "split") {
                for (const child of node.children) {
                    if (setFocus(child))
                        return true;
                }
            }
            return false;
        };
        if (setFocus(this.layout.root)) {
            this.save();
            return;
        }
        const tab = {
            id: tabId,
            type: "settings",
            title: "Settings",
            props: {},
        };
        const targetLeaf = this.findFirstLeafNotExplorer(this.layout.root) ||
            this.findFirstLeaf(this.layout.root);
        if (targetLeaf) {
            targetLeaf.tabs.push(tab);
            targetLeaf.activeTabId = tab.id;
            this.save();
        }
    }
}
export const layoutEngine = new LayoutStore();
