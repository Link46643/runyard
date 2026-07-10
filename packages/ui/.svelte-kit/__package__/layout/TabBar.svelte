<script lang="ts">
  import type { Tab } from "@runyard/common";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { appStatus } from "./appStatusStore.svelte.js";
  import { X } from "lucide-svelte";
  import Modal from "../Modal.svelte";
  import Checkbox from "../Checkbox.svelte";
  import { invoke } from "@tauri-apps/api/core";

  let { tabs, activeTabId, leafId } = $props<{ tabs: Tab[], activeTabId: string | null, leafId: string }>();

  let draggedTabId = $state<string | null>(null);

  // Save confirmation state
  let showSaveModal = $state(false);
  let pendingCloseTabId = $state<string | null>(null);
  let dontAskAgain = $state(false);

  function cleanupTerminalTab(tab: Tab) {
    if (tab.type !== "terminal") return;
    const terminalId = tab.props?.terminalId as string | undefined;
    if (!terminalId) return;

    // Kill the PTY session in Rust
    invoke("terminal_close", { id: terminalId }).catch(() => {});

    // Dispose the xterm instance from the global cache
    const cache = (window as any).__runyard_terminals as Map<string, any> | undefined;
    if (cache) {
      const entry = cache.get(terminalId);
      if (entry) {
        try { entry.fitAddon?.dispose?.(); } catch {}
        try { entry.terminal?.dispose?.(); } catch {}
        cache.delete(terminalId);
      }
    }
  }

  function handleClose(tabId: string) {
    const tab = tabs.find((t: Tab) => t.id === tabId);
    if (tab && tab.dirty && !appStatus.suppressSaveConfirmation) {
      pendingCloseTabId = tabId;
      showSaveModal = true;
    } else {
      if (tab) cleanupTerminalTab(tab);
      layoutEngine.closeTab(tabId, true);
    }
  }

  function confirmCloseWithoutSaving() {
    if (dontAskAgain) {
      appStatus.suppressSaveConfirmation = true;
    }
    if (pendingCloseTabId) {
      const tab = tabs.find((t: Tab) => t.id === pendingCloseTabId);
      if (tab) cleanupTerminalTab(tab);
      layoutEngine.closeTab(pendingCloseTabId, true);
    }
    showSaveModal = false;
    pendingCloseTabId = null;
  }

  function onDragStart(e: DragEvent, tabId: string) {
    draggedTabId = tabId;
    if (e.dataTransfer) {
      e.dataTransfer.setData("text/plain", tabId);
      e.dataTransfer.effectAllowed = "move";
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
  }

  function onDrop(e: DragEvent, targetTabId: string) {
    e.preventDefault();
    const sourceId = e.dataTransfer?.getData("text/plain");
    if (!sourceId || sourceId === targetTabId) return;

    const sourceIdx = tabs.findIndex((t: Tab) => t.id === sourceId);
    const targetIdx = tabs.findIndex((t: Tab) => t.id === targetTabId);

    if (sourceIdx !== -1 && targetIdx !== -1) {
      const newTabs = [...tabs];
      const [removed] = newTabs.splice(sourceIdx, 1);
      newTabs.splice(targetIdx, 0, removed);
      
      // Update local state and save
      tabs.length = 0;
      tabs.push(...newTabs);
      layoutEngine.save();
    }
    draggedTabId = null;
  }
</script>

<div class="tab-bar">
  <Modal
    bind:show={showSaveModal}
    title="Unsaved Changes"
    message="You have unsaved changes in this file. Are you sure you want to close it without saving?"
    confirmLabel="Discard & Close"
    cancelLabel="Cancel"
    onConfirm={confirmCloseWithoutSaving}
    onCancel={() => { showSaveModal = false; pendingCloseTabId = null; }}
  >
    <div class="modal-extra">
      <Checkbox bind:checked={dontAskAgain} label="Don't ask again" />
    </div>
  </Modal>

  {#each tabs as tab (tab.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div 
      class="tab {activeTabId === tab.id ? 'active' : ''} {draggedTabId === tab.id ? 'dragging' : ''}"
      onpointerdown={(e) => { 
        if ((e.target as HTMLElement).closest('.close-btn')) return;
        layoutEngine.setActiveTab(tab.id); 
      }}
      draggable="true"
      ondragstart={(e) => onDragStart(e, tab.id)}
      ondragover={onDragOver}
      ondrop={(e) => onDrop(e, tab.id)}
    >
      <span class="title">{tab.title}</span>
      {#if tab.dirty}<span class="dirty-dot"></span>{/if}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="close-btn" onpointerdown={(e) => { e.stopPropagation(); handleClose(tab.id); }}>
        <X size={14} strokeWidth={2} />
      </div>
    </div>
  {/each}
</div>

<style>
  .tab-bar { display: flex; background: var(--tab-bg); overflow-x: auto; flex-shrink: 0; scrollbar-width: none; border-bottom: 1px solid var(--border); }
  .tab-bar::-webkit-scrollbar { display: none; }
  .tab { padding: 8px 16px; background: var(--tab-bg); color: var(--tab-text); display: flex; align-items: center; gap: 8px; cursor: pointer; border-right: 1px solid var(--border); min-width: 120px; max-width: 240px; font-family: inherit; font-size: 13px; position: relative; user-select: none; transition: background 0.2s, color 0.2s; }
  .tab.active { background: var(--tab-active-bg); color: var(--tab-active-text); }
  .tab.dragging { opacity: 0.5; }
  .dirty-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); }
  .close-btn { background: none; border: none; color: inherit; cursor: pointer; opacity: 0; font-size: 16px; display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 4px; padding: 0; margin-left: 4px;}
  .tab:hover .close-btn, .tab.active .close-btn { opacity: 0.6; }
  .close-btn:hover { background: rgba(128,128,128,0.2); opacity: 1 !important; }
  .title { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; pointer-events: none; }

  .modal-extra {
    margin-top: 16px;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
  }
  .modal-extra label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
</style>
