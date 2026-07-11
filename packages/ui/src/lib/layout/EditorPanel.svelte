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
  import MiniMap from "./MiniMap.svelte";

  let { filePath, onDirtyChange, inlineDiff = undefined } = $props<{
    filePath: string;
    onDirtyChange: (dirty: boolean) => void;
    /** Optional inline diff annotation: line numbers for additions and deletions. */
    inlineDiff?: { additions: number[]; deletions: number[] };
  }>();

  let container: HTMLDivElement;
  let editorInstance: any = null;
  let savedContent = $state("");
  let currentContent = $state("");
  let loadError = $state<string | null>(null);
  let showWarningModal = $state(false);
  let showExternalChangeModal = $state(false);
  let warningMessage = $state("");

  // Cursor position tracking for breadcrumbs.
  let cursorLine = $state(1);
  let cursorCol = $state(1);

  // Scroll position (0–1) for minimap indicator.
  let scrollPercent = $state(0);

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

  // Breadcrumb path segments derived from the filePath prop.
  let breadcrumbSegments = $derived(() => {
    if (!filePath) return [];
    const parts = filePath.replace(/^\//, "").split("/");
    return parts;
  });

  // ── Settings-driven flags ────────────────────────────────────────────────────

  let showMinimap = $derived(settingsStore.settings.editor?.show_minimap ?? false);

  // Track the settings snapshot used to build the current editor instance so
  // we can detect when a rebuild is needed.
  let _lastVimMode = false;
  let _lastEmacsMode = false;
  let _lastStickyScroll = false;
  let _lastFoldGutter = true;
  let _lastLineWrap = false;
  let _lastFontSize = 14;
  let _lastTabSize = 2;

  // Rebuild the editor when editor-config settings that cannot be toggled via
  // a simple dispatch change (vim mode, fold gutter, line wrap, font/tab size,
  // emacs mode, sticky scroll).
  $effect(() => {
    const vimMode = settingsStore.settings.editor?.vim_mode ?? false;
    const emacsMode = settingsStore.settings.editor?.emacs_mode ?? false;
    const stickyScroll = settingsStore.settings.editor?.sticky_scroll ?? false;
    const showFoldGutter = settingsStore.settings.editor?.show_fold_gutter ?? true;
    const lineWrap = settingsStore.settings.editor?.line_wrap ?? false;
    const fontSize = settingsStore.settings.editor?.font_size ?? 14;
    const tabSize = settingsStore.settings.editor?.tab_size ?? 2;

    if (
      editorInstance &&
      (vimMode !== _lastVimMode ||
        emacsMode !== _lastEmacsMode ||
        stickyScroll !== _lastStickyScroll ||
        showFoldGutter !== _lastFoldGutter ||
        lineWrap !== _lastLineWrap ||
        fontSize !== _lastFontSize ||
        tabSize !== _lastTabSize)
    ) {
      rebuildEditor(vimMode, emacsMode, stickyScroll, showFoldGutter, lineWrap, fontSize, tabSize);
    }
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

  let _insertCmdHandler: ((e: Event) => void) | null = null;

  // Cached LSP extensions so we can reuse them across rebuilds without
  // re-initialising the language server.
  let _cachedLspExtensions: any[] = [];

  /** Tear down the current editor instance and create a fresh one using the
   *  provided settings. Preserves current document content. */
  function rebuildEditor(
    vimMode: boolean,
    emacsMode: boolean,
    stickyScroll: boolean,
    showFoldGutter: boolean,
    lineWrap: boolean,
    fontSize: number,
    tabSize: number
  ) {
    const doc = editorInstance ? editorInstance.getValue() : currentContent;
    editorInstance.destroy();
    editorInstance = null;

    editorInstance = setupEditor({
      parent: container,
      doc,
      filePath,
      lspExtensions: _cachedLspExtensions,
      fontSize,
      tabSize,
      lineWrap,
      vimMode,
      emacsMode,
      stickyScroll,
      showFoldGutter,
      inlineDiff,
      onChange: (content) => {
        currentContent = content;
      },
      onSave: (content) => {
        saveFile(content);
      },
      onSelectionChange: (line, col) => {
        cursorLine = line;
        cursorCol = col;
        appStatus.updateCursor(line, col);
      },
    });

    // Attach scroll listener to new view.
    attachScrollListener();

    _lastVimMode = vimMode;
    _lastEmacsMode = emacsMode;
    _lastStickyScroll = stickyScroll;
    _lastFoldGutter = showFoldGutter;
    _lastLineWrap = lineWrap;
    _lastFontSize = fontSize;
    _lastTabSize = tabSize;
  }

  /** Attach a DOM scroll listener to the CodeMirror scroller to update
   *  scrollPercent for the minimap indicator. */
  function attachScrollListener() {
    if (!editorInstance) return;
    const scroller = editorInstance.view.scrollDOM as HTMLElement | null;
    if (!scroller) return;
    const onScroll = () => {
      const max = scroller.scrollHeight - scroller.clientHeight;
      scrollPercent = max > 0 ? scroller.scrollTop / max : 0;
    };
    scroller.addEventListener("scroll", onScroll, { passive: true });
  }

  onDestroy(() => {
    if (_blurHandler) window.removeEventListener("blur", _blurHandler);
    if (_saveCmdHandler) document.removeEventListener("runyard:save-current-file", _saveCmdHandler);
    if (_insertCmdHandler) document.removeEventListener("runyard:insert-at-cursor", _insertCmdHandler);
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

    // Insert-at-cursor command from chat code blocks — only act when this file is the active one
    _insertCmdHandler = (e: Event) => {
      if (appStatus.activeFilePath !== filePath || !editorInstance) return;
      const text = (e as CustomEvent<{ text: string }>).detail?.text;
      if (!text) return;
      const view = editorInstance.view;
      const pos = view.state.selection.main.head;
      view.dispatch({
        changes: { from: pos, to: pos, insert: text },
        selection: { anchor: pos + text.length },
      });
      view.focus();
    };
    document.addEventListener("runyard:insert-at-cursor", _insertCmdHandler);

    // Ensure LSP settings are loaded
    if (!settingsStore.loaded) {
      await settingsStore.load();
    }

    // Build LSP extension if a language server is available
    const language = await ensureLspForFile();
    const langId = detectLanguageId(filePath);

    // Build LSP extensions if a language is detected and server is starting/ready
    if (language && langId) {
      const lspInterface = createLspInterface(language, lspStore);
      _cachedLspExtensions = createLspExtension({
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

    const initialVimMode = settingsStore.settings.editor?.vim_mode ?? false;
    const initialEmacsMode = settingsStore.settings.editor?.emacs_mode ?? false;
    const initialStickyScroll = settingsStore.settings.editor?.sticky_scroll ?? false;
    const initialFoldGutter = settingsStore.settings.editor?.show_fold_gutter ?? true;
    const initialLineWrap = settingsStore.settings.editor?.line_wrap ?? false;
    const initialFontSize = settingsStore.settings.editor?.font_size ?? 14;
    const initialTabSize = settingsStore.settings.editor?.tab_size ?? 2;

    editorInstance = setupEditor({
      parent: container,
      doc: currentContent,
      filePath,
      lspExtensions: _cachedLspExtensions,
      fontSize: initialFontSize,
      tabSize: initialTabSize,
      lineWrap: initialLineWrap,
      vimMode: initialVimMode,
      emacsMode: initialEmacsMode,
      stickyScroll: initialStickyScroll,
      showFoldGutter: initialFoldGutter,
      inlineDiff,
      onChange: (content) => {
        currentContent = content;
      },
      onSave: (content) => {
        saveFile(content);
      },
      onSelectionChange: (line, col) => {
        cursorLine = line;
        cursorCol = col;
        appStatus.updateCursor(line, col);
      },
    });

    // Record the settings used so the $effect rebuild check starts from a
    // known baseline.
    _lastVimMode = initialVimMode;
    _lastEmacsMode = initialEmacsMode;
    _lastStickyScroll = initialStickyScroll;
    _lastFoldGutter = initialFoldGutter;
    _lastLineWrap = initialLineWrap;
    _lastFontSize = initialFontSize;
    _lastTabSize = initialTabSize;

    attachScrollListener();

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

  // Minimap scroll handler — maps the clicked minimap percentage to a CM scroll.
  function handleMinimapScroll(pct: number) {
    if (!editorInstance) return;
    const scroller = editorInstance.view.scrollDOM as HTMLElement | null;
    if (!scroller) return;
    const max = scroller.scrollHeight - scroller.clientHeight;
    scroller.scrollTop = pct * max;
  }
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

  {#if filePath}
    <!-- Breadcrumb strip (1.12.6) — shown only when a file is open -->
    <div class="breadcrumb-bar">
      <div class="breadcrumb-path">
        {#each breadcrumbSegments() as seg, i}
          {#if i > 0}
            <span class="breadcrumb-sep">/</span>
          {/if}
          <button
            class="breadcrumb-seg"
            class:breadcrumb-seg--last={i === breadcrumbSegments().length - 1}
            onclick={() => {
              if (i < breadcrumbSegments().length - 1) {
                // Reconstruct the directory path up to this segment and open
                // the explorer at that location.
                const dirPath = "/" + breadcrumbSegments().slice(0, i + 1).join("/");
                layoutEngine.openExplorer?.(dirPath);
              }
            }}
          >{seg}</button>
        {/each}
      </div>
      <div class="breadcrumb-cursor">Ln {cursorLine}, Col {cursorCol}</div>
    </div>
  {/if}

  {#if loadError}
    <div class="error-overlay">
      <div class="error-icon"><TriangleAlert size={48} strokeWidth={1.5} /></div>
      <div class="error-title">Failed to load file</div>
      <div class="error-msg">{loadError}</div>
      <div class="error-path">{filePath}</div>
    </div>
  {/if}

  <div class="editor-body" style:display={loadError ? "none" : "flex"}>
    <div
      bind:this={container}
      class="editor-panel"
    ></div>

    {#if showMinimap}
      <MiniMap
        content={currentContent}
        {scrollPercent}
        onScrollTo={handleMinimapScroll}
      />
    {/if}
  </div>
</div>

<style>
  .editor-wrapper {
    width: 100%;
    height: 100%;
    position: relative;
    background-color: var(--bg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── Breadcrumb strip ── */
  .breadcrumb-bar {
    height: 24px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
  }

  .breadcrumb-path {
    display: flex;
    align-items: center;
    gap: 0;
    overflow: hidden;
    min-width: 0;
  }

  .breadcrumb-sep {
    color: var(--text-secondary);
    opacity: 0.5;
    padding: 0 2px;
    user-select: none;
  }

  .breadcrumb-seg {
    background: none;
    border: none;
    padding: 0 2px;
    font-size: 11px;
    font-family: inherit;
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 160px;
  }

  .breadcrumb-seg:hover {
    color: var(--text);
  }

  .breadcrumb-seg--last {
    color: var(--text);
    font-weight: 500;
    cursor: default;
  }

  .breadcrumb-seg--last:hover {
    color: var(--text);
  }

  .breadcrumb-cursor {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
    font-family: "JetBrains Mono", monospace;
  }

  /* ── Editor body (editor + optional minimap side by side) ── */
  .editor-body {
    flex: 1;
    display: flex;
    flex-direction: row;
    overflow: hidden;
    min-height: 0;
  }

  .editor-panel {
    flex: 1;
    min-width: 0;
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
    z-index: 1;
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

  /* 1.12.7 — Sticky scroll bar injected by the ViewPlugin */
  :global(.cm-sticky-scroll) {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10;
    height: 24px;
    line-height: 24px;
    padding: 0 10px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-family: "JetBrains Mono", "Fira Code", Consolas, monospace;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    pointer-events: none;
    user-select: none;
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
