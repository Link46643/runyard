<script lang="ts">
  import { onMount } from "svelte";
  import { Layout, layoutEngine, platform, commandRegistry, settingsStore, theme, appStatus } from "@runyard/ui";
  import { invoke } from "@tauri-apps/api/core";
  import type { Tab } from "@runyard/common";

  onMount(() => {
    platform.current = "desktop";

    // Run async initialization without returning a Promise to onMount
    (async () => {
      // ── Load persisted settings first ──────────────────────────────────────
      await settingsStore.load();

      // ── Apply saved theme ──────────────────────────────────────────────────
      const savedTheme = settingsStore.settings.appearance.theme as "light" | "dark" | undefined;
      if (savedTheme === "light" || savedTheme === "dark") {
        theme.set(savedTheme);
      }
    })();

    // ── Register all core commands ─────────────────────────────────────────
    commandRegistry.register({
      id: "terminal.new",
      title: "New Terminal",
      category: "Terminal",
      shortcut: "Ctrl+`",
      handler: () => layoutEngine.openTerminal(),
    });

    commandRegistry.register({
      id: "git.open",
      title: "Open Git Panel",
      category: "Git",
      handler: () => layoutEngine.openGit("../../"),
    });

    commandRegistry.register({
      id: "settings.open",
      title: "Open Settings",
      category: "Settings",
      shortcut: "Ctrl+,",
      handler: () => layoutEngine.openSettings(),
    });

    commandRegistry.register({
      id: "view.splitHorizontal",
      title: "Split Editor Right",
      category: "View",
      handler: () => {
        const root = layoutEngine.layout.root;
        const firstLeafId = (node: any): string | null => {
          if (node.type === "leaf") return node.id;
          if (node.type === "split") return firstLeafId(node.children[0]);
          return null;
        };
        const leafId = firstLeafId(root);
        if (leafId) layoutEngine.splitLeaf(leafId, "horizontal");
      },
    });

    commandRegistry.register({
      id: "view.splitVertical",
      title: "Split Editor Down",
      category: "View",
      handler: () => {
        const root = layoutEngine.layout.root;
        const firstLeafId = (node: any): string | null => {
          if (node.type === "leaf") return node.id;
          if (node.type === "split") return firstLeafId(node.children[0]);
          return null;
        };
        const leafId = firstLeafId(root);
        if (leafId) layoutEngine.splitLeaf(leafId, "vertical");
      },
    });

    commandRegistry.register({
      id: "view.toggleTheme",
      title: "Toggle Light/Dark Theme",
      category: "Appearance",
      handler: () => {
        theme.toggle();
        settingsStore.update("appearance", {
          theme: theme.current,
        });
      },
    });

    commandRegistry.register({
      id: "view.focusExplorer",
      title: "Focus File Explorer",
      category: "View",
      handler: () => {
        const findExplorer = (node: any): string | null => {
          if (node.type === "leaf") {
            const t = node.tabs.find((t: any) => t.type === "explorer");
            return t ? t.id : null;
          }
          if (node.type === "split") {
            for (const child of node.children) {
              const found = findExplorer(child);
              if (found) return found;
            }
          }
          return null;
        };
        const tabId = findExplorer(layoutEngine.layout.root);
        if (tabId) layoutEngine.setActiveTab(tabId);
      },
    });

    commandRegistry.register({
      id: "terminal.newCwd",
      title: "Open Terminal in Current Directory",
      category: "Terminal",
      handler: () => {
        const activePath = appStatus.activeFilePath;
        const cwd = activePath
          ? activePath.substring(0, activePath.lastIndexOf("/") || 0) || "/"
          : undefined;
        layoutEngine.openTerminal(cwd);
      },
    });

    commandRegistry.register({
      id: "tab.close",
      title: "Close Tab",
      category: "View",
      shortcut: "Ctrl+W",
      handler: () => {
        // Editor tab IDs equal the file path
        const activePath = appStatus.activeFilePath;
        if (activePath) {
          layoutEngine.closeTab(activePath);
          return;
        }
        // For non-editor tabs, let the focused pane handle it
        document.dispatchEvent(new CustomEvent("runyard:close-active-tab"));
      },
    });

    commandRegistry.register({
      id: "editor.focus",
      title: "Focus Editor",
      category: "View",
      handler: () => {
        // Focus the CodeMirror content element of the active editor
        const cmContent = document.querySelector<HTMLElement>(".cm-content");
        if (cmContent) cmContent.focus();
      },
    });

    commandRegistry.register({
      id: "file.open",
      title: "Open File",
      category: "File",
      shortcut: "Ctrl+O",
      handler: () => {
        // Focus the file explorer so the user can navigate to a file
        const findExplorer = (node: any): string | null => {
          if (node.type === "leaf") {
            const t = node.tabs.find((t: any) => t.type === "explorer");
            return t ? t.id : null;
          }
          if (node.type === "split") {
            for (const child of node.children) {
              const found = findExplorer(child);
              if (found) return found;
            }
          }
          return null;
        };
        const tabId = findExplorer(layoutEngine.layout.root);
        if (tabId) layoutEngine.setActiveTab(tabId);
      },
    });

    commandRegistry.register({
      id: "file.save",
      title: "Save Current File",
      category: "File",
      shortcut: "Ctrl+S",
      handler: () => {
        document.dispatchEvent(new CustomEvent("runyard:save-current-file"));
      },
    });

    commandRegistry.register({
      id: "file.saveAll",
      title: "Save All Open Files",
      category: "File",
      shortcut: "Ctrl+Shift+S",
      handler: () => {
        // Broadcasts to all mounted EditorPanel instances; each saves if active
        document.dispatchEvent(new CustomEvent("runyard:save-current-file"));
      },
    });

    commandRegistry.register({
      id: "tab.reopenLast",
      title: "Reopen Last Closed Tab",
      category: "View",
      shortcut: "Ctrl+Shift+T",
      handler: () => layoutEngine.reopenLastTab(),
    });

    // ── Initialize layout ─────────────────────────────────────────────────
    (async () => {
      const isLayoutEmpty = (node: any): boolean => {
        if (node.type === "leaf") return node.tabs.length === 0;
        if (node.type === "split") return node.children.every((c: any) => isLayoutEmpty(c));
        return true;
      };

      if (isLayoutEmpty(layoutEngine.layout.root)) {
        layoutEngine.clearLayout();
      }

      const root = layoutEngine.layout.root;
      if (
        root.id === "root-leaf" &&
        root.type === "leaf" &&
        root.tabs.length === 0
      ) {
        let homeDir = "../../";
        try {
          homeDir = await invoke<string>("get_home_dir");
        } catch (e) {
          console.error("Failed to get home dir", e);
        }

        const explorerTab: Tab = {
          id: "initial-explorer",
          type: "explorer",
          title: "Explorer",
          props: { workspacePath: homeDir },
        };

        const welcomeTab: Tab = {
          id: "initial-welcome",
          type: "welcome",
          title: "Welcome",
          props: {},
        };

        layoutEngine.addTab("root-leaf", explorerTab);
        layoutEngine.splitLeaf("root-leaf", "horizontal");

        const findFirstEmptyLeaf = (node: any): string | null => {
          if (node.type === "leaf" && node.tabs.length === 0) return node.id;
          if (node.type === "split") {
            for (const child of node.children) {
              const found = findFirstEmptyLeaf(child);
              if (found) return found;
            }
          }
          return null;
        };

        const emptyLeafId = findFirstEmptyLeaf(layoutEngine.layout.root);
        if (emptyLeafId) {
          layoutEngine.addTab(emptyLeafId, welcomeTab);
        }

        const newRoot = layoutEngine.layout.root;
        if (newRoot.type === "split") {
          layoutEngine.resizeLeaf(newRoot.id, [20, 80]);
        }
      }
    })();

    // ── Global keyboard shortcuts ─────────────────────────────────────────
    function handleGlobalKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "`") {
        e.preventDefault();
        commandRegistry.execute("terminal.new");
      }
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        commandRegistry.execute("settings.open");
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "T") {
        e.preventDefault();
        commandRegistry.execute("tab.reopenLast");
      }
    }
    window.addEventListener("keydown", handleGlobalKey);
    return () => window.removeEventListener("keydown", handleGlobalKey);
  });
</script>

<Layout node={layoutEngine.layout.root} />
