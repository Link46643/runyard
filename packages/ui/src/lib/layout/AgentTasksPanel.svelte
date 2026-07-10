<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { onMount, onDestroy } from "svelte";
  import { Cpu, Clock, CheckCircle, XCircle, AlertTriangle, Loader } from "lucide-svelte";

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  type AgentTaskStatus = "queued" | "running" | "awaiting_hil" | "completed" | "failed" | "cancelled";

  interface AgentTask {
    id: string;
    agent_id: string;
    agent_name: string;
    connection_id: string | null;
    session_id: string | null;
    conversation_id: string | null;
    project: string | null;
    description: string;
    status: AgentTaskStatus;
    created_at: number;
    updated_at: number;
    completed_at: number | null;
    cost_usd: number;
    current_tool: string | null;
    error: string | null;
  }

  interface AgentTaskStats {
    activeTasks: number;
    totalTasks: number;
    totalCostUsd: number;
    hilPending: number;
  }

  let tasks = $state<AgentTask[]>([]);
  let stats = $state<AgentTaskStats>({ activeTasks: 0, totalTasks: 0, totalCostUsd: 0, hilPending: 0 });
  let expandedErrors = $state<Set<string>>(new Set());
  let now = $state(Date.now());

  const COLUMNS: { key: AgentTaskStatus; label: string }[] = [
    { key: "queued",       label: "Queued" },
    { key: "running",      label: "Running" },
    { key: "awaiting_hil", label: "Awaiting HIL" },
    { key: "completed",    label: "Completed" },
    { key: "failed",       label: "Failed" },
  ];

  function tasksForStatus(status: AgentTaskStatus) {
    return tasks.filter((t) => t.status === status);
  }

  function statusDotColor(status: AgentTaskStatus): string {
    switch (status) {
      case "running":      return "var(--text-warning)";
      case "awaiting_hil": return "var(--accent-warning)";
      case "completed":    return "var(--text-success)";
      case "failed":       return "var(--text-error)";
      case "queued":
      case "cancelled":
      default:             return "var(--text-tertiary)";
    }
  }

  function formatElapsed(createdAt: number): string {
    const ms = now - createdAt;
    const s = Math.floor(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    if (m < 60) return `${m}m ${s % 60}s`;
    const h = Math.floor(m / 60);
    return `${h}h ${m % 60}m`;
  }

  function toggleError(id: string) {
    const next = new Set(expandedErrors);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedErrors = next;
  }

  async function loadAll() {
    try {
      const [t, s] = await Promise.all([
        invoke<AgentTask[]>("agent_task_list"),
        invoke<AgentTaskStats>("agent_task_stats"),
      ]);
      tasks = t;
      stats = s;
    } catch (e) {
      console.error("[AgentTasksPanel] Failed to load tasks", e);
    }
  }

  async function updateStatus(id: string, status: AgentTaskStatus, extra?: { currentTool?: string; error?: string; costUsd?: number }) {
    try {
      await invoke("agent_task_update_status", {
        id,
        status,
        currentTool: extra?.currentTool ?? null,
        error: extra?.error ?? null,
        costUsd: extra?.costUsd ?? null,
      });
      await loadAll();
    } catch (e) {
      console.error("[AgentTasksPanel] Failed to update task status", e);
    }
  }

  async function deleteTask(id: string) {
    try {
      await invoke("agent_task_delete", { id });
      await loadAll();
    } catch (e) {
      console.error("[AgentTasksPanel] Failed to delete task", e);
    }
  }

  async function clearCompleted() {
    try {
      await invoke<number>("agent_task_clear_completed");
      await loadAll();
    } catch (e) {
      console.error("[AgentTasksPanel] Failed to clear completed", e);
    }
  }

  function handleToolCall(event: Event) {
    const e = event as CustomEvent<{ taskId?: string; agentId?: string; tool?: string }>;
    if (!e.detail) return;
    const { taskId, agentId, tool } = e.detail;
    tasks = tasks.map((t) => {
      if ((taskId && t.id === taskId) || (agentId && t.agent_id === agentId)) {
        return { ...t, current_tool: tool ?? t.current_tool };
      }
      return t;
    });
  }

  let pollInterval: ReturnType<typeof setInterval>;
  let clockInterval: ReturnType<typeof setInterval>;

  onMount(() => {
    loadAll();
    pollInterval = setInterval(loadAll, 5000);
    clockInterval = setInterval(() => { now = Date.now(); }, 1000);
    window.addEventListener("acp:tool_call", handleToolCall);
  });

  onDestroy(() => {
    clearInterval(pollInterval);
    clearInterval(clockInterval);
    window.removeEventListener("acp:tool_call", handleToolCall);
  });
</script>

<div class="agent-tasks-panel">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">AGENT TASKS</span>
    <span class="stats-strip">
      {stats.activeTasks} active
      · ${stats.totalCostUsd.toFixed(4)} total
      · {stats.hilPending} pending approval
    </span>
    <button class="ghost-btn" onclick={clearCompleted}>Clear completed</button>
  </div>

  <!-- Kanban board -->
  <div class="kanban-board">
    {#each COLUMNS as col}
      {@const colTasks = tasksForStatus(col.key)}
      <div class="kanban-column" class:hil-column={col.key === "awaiting_hil"}>
        <!-- Column header -->
        <div class="column-header">
          <span
            class="status-dot"
            style="background: {statusDotColor(col.key)};"
          ></span>
          <span class="column-title">{col.label}</span>
          <span class="column-count">{colTasks.length}</span>
        </div>

        <!-- Cards -->
        <div class="column-cards">
          {#if colTasks.length === 0}
            <div class="empty-column">—</div>
          {:else}
            {#each colTasks as task (task.id)}
              <div class="task-card" class:failed-card={task.status === "failed"}>
                <!-- Card top row: agent name + status dot -->
                <div class="card-top">
                  <span class="agent-name">{task.agent_name}</span>
                  <span
                    class="status-dot"
                    style="background: {statusDotColor(task.status)};"
                  ></span>
                </div>

                <!-- Description -->
                <div class="task-description">{task.description}</div>

                <!-- Current tool (running only) -->
                {#if task.status === "running" && task.current_tool}
                  <div class="current-tool">⚙ {task.current_tool}</div>
                {/if}

                <!-- Error (failed) -->
                {#if task.status === "failed" && task.error}
                  <button
                    class="error-toggle"
                    onclick={() => toggleError(task.id)}
                  >
                    {expandedErrors.has(task.id) ? "Hide error" : "Show error"}
                  </button>
                  {#if expandedErrors.has(task.id)}
                    <div class="error-text">{task.error}</div>
                  {/if}
                {/if}

                <!-- Bottom row -->
                <div class="card-bottom">
                  <span class="task-project">{task.project ?? "—"}</span>
                  <span class="task-elapsed">{formatElapsed(task.created_at)}</span>
                  <span class="task-cost">${task.cost_usd.toFixed(4)}</span>
                </div>

                <!-- Action buttons -->
                <div class="card-actions">
                  {#if task.status === "awaiting_hil"}
                    <button
                      class="action-btn approve-btn"
                      onclick={() => updateStatus(task.id, "running")}
                    >
                      Approve
                    </button>
                  {/if}
                  {#if task.status === "running" || task.status === "queued"}
                    <button
                      class="action-btn cancel-btn"
                      onclick={() => updateStatus(task.id, "cancelled")}
                    >
                      Cancel
                    </button>
                  {/if}
                  {#if task.status === "completed" || task.status === "failed" || task.status === "cancelled"}
                    <button
                      class="ghost-btn small-ghost"
                      onclick={() => deleteTask(task.id)}
                    >
                      Delete
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .agent-tasks-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }

  /* Header */
  .panel-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .panel-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .stats-strip {
    font-size: 11px;
    color: var(--text-tertiary);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ghost-btn {
    font-size: 11px;
    color: var(--text-secondary);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-sans);
  }

  .ghost-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-elevated);
  }

  /* Kanban board */
  .kanban-board {
    display: flex;
    flex-direction: row;
    gap: 0;
    overflow-x: auto;
    flex: 1;
    min-height: 0;
  }

  .kanban-column {
    display: flex;
    flex-direction: column;
    min-width: 220px;
    flex: 1;
    border-right: 1px solid var(--border);
    background: var(--bg);
    overflow: hidden;
  }

  .kanban-column:last-child {
    border-right: none;
  }

  .hil-column {
    border: 1px solid var(--accent-warning);
  }

  .column-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .column-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
    flex: 1;
  }

  .column-count {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .column-cards {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .empty-column {
    font-size: 11px;
    color: var(--text-tertiary);
    text-align: center;
    padding: 16px 0;
  }

  /* Task card */
  .task-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .failed-card {
    border-color: var(--border-error);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .agent-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }

  .task-description {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .current-tool {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .error-toggle {
    font-size: 11px;
    color: var(--text-error);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    font-family: var(--font-sans);
    text-decoration: underline;
  }

  .error-text {
    font-size: 11px;
    color: var(--text-error);
    font-family: var(--font-mono);
    white-space: pre-wrap;
    word-break: break-all;
    background: var(--bg);
    border: 1px solid var(--border-error);
    border-radius: var(--radius-1);
    padding: 4px 6px;
    max-height: 80px;
    overflow-y: auto;
  }

  .card-bottom {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 2px;
  }

  .task-project {
    font-size: 11px;
    color: var(--text-tertiary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .task-elapsed {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .task-cost {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .card-actions {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .action-btn {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: var(--radius-1);
    border: none;
    cursor: pointer;
    font-family: var(--font-sans);
    font-weight: 500;
  }

  .approve-btn {
    background: var(--accent-success);
    color: var(--bg);
  }

  .approve-btn:hover {
    opacity: 0.85;
  }

  .cancel-btn {
    background: var(--accent-danger);
    color: var(--bg);
  }

  .cancel-btn:hover {
    opacity: 0.85;
  }

  .small-ghost {
    font-size: 11px;
    padding: 2px 6px;
  }
</style>
