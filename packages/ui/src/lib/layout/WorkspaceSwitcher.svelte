<script lang="ts">
  import { onMount } from "svelte";
  import { FolderOpen, X } from "lucide-svelte";
  import { workspaceStore } from "../stores/workspaceStore.svelte.js";

  let { onClose = () => {} }: { onClose?: () => void } = $props();

  const recents = $derived(workspaceStore.recentWorkspaces);

  $effect(() => { workspaceStore.loadRecent(); });

  function relativeTime(ts: number): string {
    const diff = Date.now() - ts;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    return `${Math.floor(hrs / 24)}d ago`;
  }

  async function selectWorkspace(path: string) {
    await workspaceStore.open(path);
    onClose();
  }

  async function openFolder() {
    await workspaceStore.openViaDialog();
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeydown);
    return () => document.removeEventListener("keydown", handleKeydown);
  });
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="switcher-backdrop" onclick={onClose}>
  <div class="switcher-panel" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" aria-label="Switch workspace">
    <div class="switcher-header">
      <span class="switcher-title">SWITCH WORKSPACE</span>
      <button class="close-btn" onclick={onClose} aria-label="Close"><X size={14} strokeWidth={1.5} /></button>
    </div>
    <div class="switcher-body">
      {#if recents.length === 0}
        <p class="empty-msg">No recent workspaces.</p>
      {:else}
        {#each recents as ws (ws.path)}
          <button class="workspace-row" class:active={ws.path === workspaceStore.currentPath} onclick={() => selectWorkspace(ws.path)}>
            <div class="ws-info">
              <span class="ws-name">{ws.name}</span>
              <span class="ws-path">{ws.path}</span>
            </div>
            <span class="ws-time">{relativeTime(ws.last_opened_at)}</span>
          </button>
        {/each}
      {/if}
      <div class="switcher-footer">
        <button class="open-folder-btn" onclick={openFolder}><FolderOpen size={13} strokeWidth={1.5} />Open folder...</button>
      </div>
    </div>
  </div>
</div>

<style>
  .switcher-backdrop { position:fixed;inset:0;z-index:300;background:transparent;display:flex;align-items:flex-start;justify-content:center;padding-top:60px; }
  .switcher-panel { width:320px;background:var(--bg-elevated);border:1px solid var(--border);border-radius:4px;box-shadow:var(--shadow-2);overflow:hidden; }
  .switcher-header { display:flex;align-items:center;justify-content:space-between;padding:8px 12px;border-bottom:1px solid var(--border); }
  .switcher-title { font-size:11px;font-weight:700;letter-spacing:0.05em;color:var(--text-secondary);text-transform:uppercase; }
  .close-btn { background:none;border:none;color:var(--text-secondary);cursor:pointer;padding:2px;display:flex;align-items:center;border-radius:2px; }
  .close-btn:hover { background:var(--bg-tertiary);color:var(--text); }
  .switcher-body { max-height:360px;overflow-y:auto; }
  .empty-msg { font-size:12px;color:var(--text-tertiary);text-align:center;padding:20px;margin:0; }
  .workspace-row { display:flex;align-items:center;gap:8px;padding:8px 12px;background:transparent;border:none;border-bottom:1px solid var(--border);cursor:pointer;text-align:left;width:100%; }
  .workspace-row:hover { background:var(--bg-tertiary); }
  .workspace-row.active { background:color-mix(in srgb,var(--accent) 8%,transparent); }
  .ws-info { flex:1;min-width:0;display:flex;flex-direction:column;gap:1px; }
  .ws-name { font-size:13px;font-weight:500;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap; }
  .ws-path { font-size:11px;color:var(--text-tertiary);font-family:var(--font-mono);overflow:hidden;text-overflow:ellipsis;white-space:nowrap; }
  .ws-time { font-size:11px;color:var(--text-tertiary);flex-shrink:0; }
  .switcher-footer { border-top:1px solid var(--border);padding:8px 12px; }
  .open-folder-btn { display:inline-flex;align-items:center;gap:6px;font-size:12px;color:var(--text-secondary);background:none;border:none;cursor:pointer;padding:4px 0;border-radius:2px; }
  .open-folder-btn:hover { color:var(--text); }
</style>
