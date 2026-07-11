<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { onMount, onDestroy } from "svelte";
  import { Cpu, Clock, CheckCircle, XCircle, AlertTriangle, Loader, Bell, BellOff } from "lucide-svelte";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
  import { acpStore } from "../stores/acpStore.svelte.js";

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

  // Task 1: search/filter
  let searchQuery = $state("");

  // Task 4: notifications
  let notificationsEnabled = $state(
    typeof localStorage !== "undefined"
      ? localStorage.getItem("runyard:agent-notifications") !== "false"
      : true
  );
  let prevHilPending = $state(0);
  let prevTaskStatuses = $state<Record<string, AgentTaskStatus>>({});

  // Task 5: graph toggle
  let showGraph = $state(false);

  const COLUMNS: { key: AgentTaskStatus; label: string }[] = [
    { key: "queued",       label: "Queued" },
    { key: "running",      label: "Running" },
    { key: "awaiting_hil", label: "Awaiting HIL" },
    { key: "completed",    label: "Completed" },
    { key: "failed",       label: "Failed" },
  ];

  // Task 1: filtered tasks
  let filteredTasks = $derived(
    searchQuery.trim() === ""
      ? tasks
      : tasks.filter((t) => {
          const q = searchQuery.toLowerCase();
          return (
            t.description.toLowerCase().includes(q) ||
            t.agent_name.toLowerCase().includes(q) ||
            (t.project ?? "").toLowerCase().includes(q)
          );
        })
  );

  function tasksForStatus(status: AgentTaskStatus) {
    return filteredTasks.filter((t) => t.status === status);
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

  // Task 2: derived stats
  let avgResponseSecs = $derived(() => {
    const completed = tasks.filter((t) => t.status === "completed" && t.completed_at !== null);
    if (completed.length === 0) return null;
    const total = completed.reduce((sum, t) => sum + (t.completed_at! - t.created_at), 0);
    return Math.round(total / completed.length / 1000);
  });

  let runningCount = $derived(tasks.filter((t) => t.status === "running").length);
  let queuedCount = $derived(tasks.filter((t) => t.status === "queued").length);

  // Task 4: notification helper
  async function tryNotify(title: string, body?: string) {
    if (!notificationsEnabled) return;
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      if (granted) {
        sendNotification({ title, body });
      }
    } catch {
      // silently ignore
    }
  }

  function toggleNotifications() {
    notificationsEnabled = !notificationsEnabled;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("runyard:agent-notifications", notificationsEnabled ? "true" : "false");
    }
  }

  // Task 1: export tasks as JSON
  function exportTasks() {
    const date = new Date().toISOString().slice(0, 10);
    const json = JSON.stringify(tasks, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `runyard-agent-tasks-${date}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }

  async function loadAll() {
    try {
      const [t, s] = await Promise.all([
        invoke<AgentTask[]>("agent_task_list"),
        invoke<AgentTaskStats>("agent_task_stats"),
      ]);

      // Task 4: detect HIL increase
      if (s.hilPending > prevHilPending) {
        tryNotify("Agent task awaiting approval", `${s.hilPending} task(s) need your approval`);
      }
      prevHilPending = s.hilPending;

      // Task 4: detect status transitions
      const newStatuses: Record<string, AgentTaskStatus> = {};
      for (const task of t) {
        newStatuses[task.id] = task.status;
        const prev = prevTaskStatuses[task.id];
        if (prev && prev !== task.status) {
          if (task.status === "completed") {
            tryNotify("Agent task completed", task.description);
          } else if (task.status === "failed") {
            tryNotify("Agent task failed", task.description);
          }
        }
      }
      prevTaskStatuses = newStatuses;

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

  // Task 3: retry failed task
  async function retryTask(id: string) {
    try {
      await invoke("agent_task_update_status", {
        id,
        status: "queued",
        currentTool: null,
        error: null,
        costUsd: null,
      });
      await loadAll();
    } catch (e) {
      console.error("[AgentTasksPanel] Failed to retry task", e);
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

  // Task 5: graph layout helpers
  let svgWidth = $state(600);
  let svgRef = $state<SVGElement | null>(null);

  let activeAgents = $derived(
    acpStore.agents.filter((a) => {
      const status = (a as any).status;
      return status === "connected" || status === "ready";
    })
  );

  let runningTasks = $derived(tasks.filter((t) => t.status === "running"));

  let svgH = $derived(Math.max(200, activeAgents.length * 60 + 80));

  function graphAgentY(index: number, total: number, height: number): number {
    if (total === 0) return height / 2;
    const margin = 40;
    const step = (height - margin * 2) / Math.max(total - 1, 1);
    return margin + index * step;
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

    <!-- Task 2: improved stats strip -->
    <span class="stats-strip">
      <span style="color: var(--text-warning);">{runningCount} running</span>
      {" · "}
      <span style="color: var(--text-tertiary);">{queuedCount} queued</span>
      {" · "}
      <span style="color: var(--accent-warning);">{stats.hilPending} pending HIL</span>
      {" · "}
      <span style="color: var(--text-tertiary);">${stats.totalCostUsd.toFixed(2)}</span>
      {#if avgResponseSecs() !== null}
        {" · "}
        <span style="color: var(--text-tertiary);">avg {avgResponseSecs()}s</span>
      {/if}
    </span>

    <!-- Task 5: graph toggle -->
    <button class="ghost-btn" onclick={() => { showGraph = !showGraph; }}>
      {showGraph ? "Board" : "Graph"}
    </button>

    <!-- Task 4: notification toggle -->
    <button
      class="ghost-btn icon-btn"
      title={notificationsEnabled ? "Disable notifications" : "Enable notifications"}
      onclick={toggleNotifications}
    >
      {#if notificationsEnabled}
        <Bell size={13} />
      {:else}
        <BellOff size={13} />
      {/if}
    </button>

    <!-- Task 1: export button -->
    <button class="ghost-btn" onclick={exportTasks}>Export</button>

    <button class="ghost-btn" onclick={clearCompleted}>Clear completed</button>
  </div>

  <!-- Task 1: search input -->
  <div class="search-bar">
    <input
      class="search-input"
      placeholder="Search tasks..."
      bind:value={searchQuery}
    />
  </div>

  {#if showGraph}
    <!-- Task 5: Agent routing graph -->
    <div class="graph-container">
      <svg
        bind:this={svgRef}
        class="routing-graph"
        width="100%"
        style="min-height: 200px;"
        viewBox="0 0 600 {svgH}"
        preserveAspectRatio="xMidYMid meet"
      >
        <!-- User node -->
        <circle cx="60" cy={svgH / 2} r="24" fill="var(--bg-tertiary)" stroke="var(--border-active)" stroke-width="1.5" />
        <text x="60" y={svgH / 2 + 4} text-anchor="middle" fill="var(--text-secondary)" font-size="11" font-family="var(--font-mono)">user</text>

        {#each activeAgents as agent, i}
          {@const ay = graphAgentY(i, activeAgents.length, svgH)}
          <!-- Arrow from user to agent if there's a running task for this agent -->
          {#if runningTasks.some((t) => t.agent_id === (agent as any).id || t.agent_name === (agent as any).name)}
            <line
              x1="84" y1={svgH / 2}
              x2="476" y2={ay}
              stroke="var(--accent)"
              stroke-width="1.5"
              marker-end="url(#arrowhead)"
            />
          {:else}
            <line
              x1="84" y1={svgH / 2}
              x2="476" y2={ay}
              stroke="var(--border)"
              stroke-width="1"
              stroke-dasharray="4 3"
            />
          {/if}
          <!-- Agent node -->
          <circle cx="500" cy={ay} r="24" fill="var(--bg-tertiary)" stroke="var(--border-active)" stroke-width="1.5" />
          <text x="500" y={ay + 4} text-anchor="middle" fill="var(--text-secondary)" font-size="11" font-family="var(--font-mono)">
            {(agent as any).name?.slice(0, 6) ?? "agent"}
          </text>
          <!-- Agent label below node -->
          <text x="500" y={ay + 38} text-anchor="middle" fill="var(--text-tertiary)" font-size="10">
            {(agent as any).name ?? ""}
          </text>
        {/each}

        {#if activeAgents.length === 0}
          <text x="300" y={svgH / 2} text-anchor="middle" fill="var(--text-tertiary)" font-size="12">
            No connected agents
          </text>
        {/if}

        <!-- Arrow marker definition -->
        <defs>
          <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
            <polygon points="0 0, 8 3, 0 6" fill="var(--accent)" />
          </marker>
        </defs>
      </svg>
    </div>
  {:else}
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
                    <!-- Task 3: Retry button for failed tasks -->
                    {#if task.status === "failed"}
                      <button
                        class="action-btn retry-btn"
                        onclick={() => retryTask(task.id)}
                      >
                        Retry
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
  {/if}
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
    gap: 6px;
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
    border-radius: 2px;
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

  .icon-btn {
    padding: 2px 5px;
    display: flex;
    align-items: center;
  }

  /* Search bar */
  .search-bar {
    padding: 5px 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .search-input {
    width: 100%;
    box-sizing: border-box;
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 3px 8px;
    color: var(--text);
    font-family: var(--font-sans);
    outline: none;
  }

  .search-input:focus {
    border-color: var(--border-active);
  }

  .search-input::placeholder {
    color: var(--text-tertiary);
  }

  /* Graph */
  .graph-container {
    flex: 1;
    overflow: auto;
    background: var(--bg);
    padding: 8px;
  }

  .routing-graph {
    display: block;
    width: 100%;
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
    flex-wrap: wrap;
  }

  .action-btn {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 2px;
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

  /* Task 3: retry button */
  .retry-btn {
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 2px;
    cursor: pointer;
    font-family: var(--font-sans);
    font-weight: 500;
  }

  .retry-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-tertiary);
  }

  .small-ghost {
    font-size: 11px;
    padding: 2px 6px;
  }
</style>
