<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { setupEditor } from "@runyard/editor";
  import { createLspExtension, createLspInterface, detectLanguageId, pathToUri } from "@runyard/editor/lsp";
  import { appStatus } from "./appStatusStore.svelte.js";
  import { lspStore } from "./lspStore.svelte.js";
  import { settingsStore } from "./settingsStore.svelte.js";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { TriangleAlert } from "lucide-svelte";
  import Modal from "../Modal.svelte";

  let { filePath, onDirtyChange } = $props<{
    filePath: string;
    onDirtyChange: (dirty: boolean) => void;
  }>();

  let container: HTMLDivElement;
  let editorInstance: any = null;
  let savedContent = $state("");
  let currentContent = $state("");
  let loadError = $state<string | null>(null);
  let showWarningModal = $state(false);
  let showExternalChangeModal = $state(false);
  let warningMessage = $state("");

  // Flag to ignore fs:changed events triggered by our own writes
  let ignoringNextChange = false;

  // Refs held so onDestroy can clean up after the async onMount
  let _blurHandler: (() => void) | null = null;
  let _saveCmdHandler: (() => void) | null = null;
  let _unlistenFs: (() => void) | null = null;

  let isDirty = $derived(savedContent !== currentContent && savedContent !== "");

  $effect(() => {
    onDirtyChange(isDirty);
  });

  async function loadFile(silent = false) {
    try {
      loadError = null;
      const rawContent = await invoke<string>("fs_read", { path: filePath });
      const content = rawContent.replace(/\r\n/g, "\n");

      if (!silent) {
        currentContent = content;
        savedContent = content;
        if (editorInstance) {
          editorInstance.setValue(content);
        }
      } else {
        savedContent = content;
      }
    } catch (e) {
      if (!silent) {
        warningMessage = `${e}`;
        if (appStatus.consumeJustOpened(filePath)) {
          showWarningModal = true;
        } else {
          loadError = warningMessage;
        }
      }
    }
  }

  async function saveFile(content: string) {
    if (loadError) return;
    try {
      ignoringNextChange = true;
      await invoke("fs_write", { path: filePath, contents: content });
      savedContent = content;
      currentContent = content;
      setTimeout(() => {
        ignoringNextChange = false;
      }, 100);
    } catch (e) {
      ignoringNextChange = false;
      console.error("[EditorPanel] Failed to write file", e);
    }
  }

  // ── LSP: start language server and build extension ────────────────────────

  async function ensureLspForFile() {
    const langId = detectLanguageId(filePath);
    if (!langId) return null;

    const langKey = langId === "javascript" ? "typescript" : langId;
    const langConfig = settingsStore.settings.lsp[langKey as keyof typeof settingsStore.settings.lsp];
    if (!langConfig?.enabled) return null;

    // Start server if not running
    const currentStatus = lspStore.getStatus(langKey);
    if (currentStatus === "disconnected") {
      // Guess workspace path from file path
      const workspacePath = filePath.substring(0, filePath.lastIndexOf("/") || 0) || "/";
      await lspStore.start(langKey, workspacePath, langConfig.path_override ?? undefined);
    }

    return langKey;
  }

  async function initializeLsp(language: string) {
    // Send LSP initialize request
    const rootUri = "file://" + (filePath.substring(0, filePath.lastIndexOf("/")) || "/");
    const initMsg = {
      jsonrpc: "2.0",
      id: 0,
      method: "initialize",
      params: {
        processId: null,
        rootUri,
        capabilities: {
          textDocument: {
            synchronization: {
              didOpen: true,
              didChange: true,
              didClose: true,
            },
            completion: {
              completionItem: {
                snippetSupport: false,
                documentationFormat: ["plaintext"],
              },
            },
            hover: {
              contentFormat: ["plaintext"],
            },
            publishDiagnostics: { relatedInformation: false },
            definition: {},
            formatting: {},
          },
          workspace: {
            workspaceFolders: true,
          },
        },
        workspaceFolders: null,
      },
    };

    await lspStore.send(language, initMsg);
  }

  onDestroy(() => {
    if (_blurHandler) window.removeEventListener("blur", _blurHandler);
    if (_saveCmdHandler) document.removeEventListener("runyard:save-current-file", _saveCmdHandler);
    if (editorInstance) editorInstance.destroy();
    appStatus.updateActiveFile(null);
    appStatus.updateCursor(1, 1);
    if (_unlistenFs) _unlistenFs();
  });

  onMount(async () => {
    _blurHandler = () => {
      if (isDirty) {
        saveFile(currentContent);
      }
    };
    window.addEventListener("blur", _blurHandler);

    // Global save command — only act when this file is the active one
    _saveCmdHandler = () => {
      if (!loadError && appStatus.activeFilePath === filePath) {
        saveFile(currentContent);
      }
    };
    document.addEventListener("runyard:save-current-file", _saveCmdHandler);

    // Ensure LSP settings are loaded
    if (!settingsStore.loaded) {
      await settingsStore.load();
    }

    // Build LSP extension if a language server is available
    const language = await ensureLspForFile();
    const langId = detectLanguageId(filePath);

    // Build LSP extensions if a language is detected and server is starting/ready
    let lspExtensions: any[] = [];
    if (language && langId) {
      const lspInterface = createLspInterface(language, lspStore);
      lspExtensions = createLspExtension({
        lsp: lspInterface,
        fileUri: pathToUri(filePath),
        languageId: langId,
        filePath,
        formatOnSave: settingsStore.settings.editor.format_on_save,
        onGoToDefinition: (targetPath, line, col) => {
          // Open the target file in an editor tab
          layoutEngine.openEditor(targetPath, targetPath.split("/").pop() ?? targetPath);
          // TODO: scroll to position (requires post-open callback)
        },
      });

      // Initialize LSP if this is the first time we're connecting
      const status = lspStore.getStatus(language);
      if (status === "starting") {
        // Wait briefly then send initialize
        setTimeout(() => initializeLsp(language), 500);
      } else if (status === "ready") {
        // Already initialized — didOpen will be sent by the ViewPlugin
      }
    }

    editorInstance = setupEditor({
      parent: container,
      doc: currentContent,
      filePath,
      lspExtensions,
      fontSize: settingsStore.settings.editor.font_size || 14,
      tabSize: settingsStore.settings.editor.tab_size || 2,
      lineWrap: settingsStore.settings.editor.line_wrap ?? false,
      onChange: (content) => {
        currentContent = content;
      },
      onSave: (content) => {
        saveFile(content);
      },
      onSelectionChange: (line, col) => {
        appStatus.updateCursor(line, col);
      },
    });

    await loadFile();
    appStatus.updateActiveFile(filePath);

    // External change listener — store unlisten fn for onDestroy
    _unlistenFs = await listen<string>("fs:changed", (event) => {
      if (event.payload === filePath) {
        if (ignoringNextChange) return;
        if (!isDirty) {
          loadFile();
        } else {
          showExternalChangeModal = true;
        }
      }
    });
  });
</script>

<div class="editor-wrapper">
  <Modal
    bind:show={showWarningModal}
    title="Warning"
    message={"Failed to read file correctly. It might be binary or have an invalid encoding.\n\n" +
      warningMessage}
    confirmLabel="Open Anyway"
    onConfirm={() => {
      showWarningModal = false;
      loadError = null;
    }}
    onCancel={() => {
      showWarningModal = false;
      layoutEngine.closeTab(filePath);
    }}
  />

  <Modal
    bind:show={showExternalChangeModal}
    title="File Changed Externally"
    message={`The file "${filePath}" has been modified by another program. Your unsaved changes may be lost if you reload. Reload?`}
    confirmLabel="Reload File"
    cancelLabel="Keep My Changes"
    onConfirm={() => {
      showExternalChangeModal = false;
      loadFile();
    }}
    onCancel={() => {
      showExternalChangeModal = false;
    }}
  />

  {#if loadError}
    <div class="error-overlay">
      <div class="error-icon"><TriangleAlert size={48} strokeWidth={1.5} /></div>
      <div class="error-title">Failed to load file</div>
      <div class="error-msg">{loadError}</div>
      <div class="error-path">{filePath}</div>
    </div>
  {/if}
  <div
    bind:this={container}
    class="editor-panel"
    style:display={loadError ? "none" : "block"}
  ></div>
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    position: relative;
    background-color: var(--bg);
  }

  .editor-panel {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--bg);
  }

  .error-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background-color: var(--bg);
    color: var(--text-secondary);
    padding: 20px;
    text-align: center;
  }

  .error-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .error-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text);
    margin-bottom: 8px;
  }

  .error-msg {
    font-size: 14px;
    margin-bottom: 16px;
    max-width: 600px;
    word-break: break-word;
  }

  .error-path {
    font-family: "JetBrains Mono", monospace;
    font-size: 12px;
    background: var(--bg-secondary);
    padding: 4px 8px;
    border-radius: 4px;
    border: 1px solid var(--border);
  }

  :global(.cm-editor) {
    height: 100%;
  }
  :global(.cm-scroller) {
    font-family: "JetBrains Mono", "Fira Code", Consolas, monospace;
    /* --editor-font-size is set dynamically by setupEditor() from settings */
    font-size: var(--editor-font-size, 14px);
  }
  :global(.cm-scroller::-webkit-scrollbar) {
    width: 10px;
    height: 10px;
  }
  :global(.cm-scroller::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(.cm-scroller::-webkit-scrollbar-thumb) {
    background: rgba(128, 128, 128, 0.2);
  }
  :global(.cm-scroller::-webkit-scrollbar-thumb:hover) {
    background: rgba(128, 128, 128, 0.3);
  }
</style>
