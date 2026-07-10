<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { onMount } from "svelte";

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  interface Todo {
    id: string;
    workspace_path: string;
    text: string;
    is_done: boolean;
    sort_order: number;
    created_at: number;
    updated_at: number;
  }

  const WORKSPACE_PATH = "../../";

  let todos = $state<Todo[]>([]);
  let newItemText = $state("");
  let filter = $state<"all" | "active" | "done">("all");

  // Drag-and-drop state
  let dragSourceId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);

  let total = $derived(todos.length);
  let done = $derived(todos.filter((t) => t.is_done).length);

  let filtered = $derived.by(() => {
    let list = [...todos];
    if (filter === "active") list = list.filter((t) => !t.is_done);
    if (filter === "done")   list = list.filter((t) => t.is_done);
    // Completed items pushed to bottom within the filtered view
    list.sort((a, b) => {
      if (a.is_done !== b.is_done) return a.is_done ? 1 : -1;
      return a.sort_order - b.sort_order;
    });
    return list;
  });

  async function loadTodos() {
    try {
      const list = await invoke<Todo[]>("todo_list", { workspacePath: WORKSPACE_PATH });
      todos = list;
    } catch (e) {
      console.error("[TodoPanel] Failed to load todos", e);
    }
  }

  async function addTodo(text: string) {
    const trimmed = text.trim();
    if (!trimmed) return;
    try {
      const created = await invoke<Todo>("todo_create", { workspacePath: WORKSPACE_PATH, text: trimmed });
      todos = [...todos, created];
    } catch (e) {
      console.error("[TodoPanel] Failed to create todo", e);
    }
  }

  async function toggleTodo(id: string) {
    const todo = todos.find((t) => t.id === id);
    if (!todo) return;
    try {
      const updated = await invoke<Todo>("todo_update", { id, isDone: !todo.is_done });
      todos = todos.map((t) => (t.id === id ? updated : t));
    } catch (e) {
      console.error("[TodoPanel] Failed to update todo", e);
    }
  }

  async function deleteTodo(id: string) {
    try {
      await invoke("todo_delete", { id });
      todos = todos.filter((t) => t.id !== id);
    } catch (e) {
      console.error("[TodoPanel] Failed to delete todo", e);
    }
  }

  async function reorderTodos(orderedIds: string[]) {
    try {
      await invoke("todo_reorder", { workspacePath: WORKSPACE_PATH, orderedIds });
      // Apply local order immediately
      const idxMap = new Map(orderedIds.map((id, i) => [id, i]));
      todos = [...todos].sort((a, b) => {
        const ai = idxMap.get(a.id) ?? a.sort_order;
        const bi = idxMap.get(b.id) ?? b.sort_order;
        return ai - bi;
      }).map((t, i) => ({ ...t, sort_order: i }));
    } catch (e) {
      console.error("[TodoPanel] Failed to reorder todos", e);
    }
  }

  // New item input handler
  function onNewItemKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      addTodo(newItemText);
      newItemText = "";
    }
  }

  // Per-item keyboard handler
  function onItemKeydown(e: KeyboardEvent, todo: Todo, index: number) {
    const input = e.currentTarget as HTMLInputElement;

    if (e.key === "Enter") {
      e.preventDefault();
      // Add a new item below this one
      addTodo("");
    } else if ((e.key === "Delete" || e.key === "Backspace") && input.value === "") {
      e.preventDefault();
      deleteTodo(todo.id);
      // Focus the item above
      setTimeout(() => {
        const items = document.querySelectorAll<HTMLInputElement>(".todo-text-input");
        const target = items[Math.max(0, index - 1)];
        target?.focus();
      }, 50);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const items = document.querySelectorAll<HTMLInputElement>(".todo-text-input");
      (items[index - 1] as HTMLInputElement | undefined)?.focus();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const items = document.querySelectorAll<HTMLInputElement>(".todo-text-input");
      (items[index + 1] as HTMLInputElement | undefined)?.focus();
    }
  }

  async function onItemBlur(todo: Todo, value: string) {
    const trimmed = value.trim();
    if (trimmed === todo.text) return;
    if (!trimmed) {
      deleteTodo(todo.id);
      return;
    }
    try {
      const updated = await invoke<Todo>("todo_update_text", { id: todo.id, text: trimmed });
      todos = todos.map((t) => (t.id === todo.id ? updated : t));
    } catch (e) {
      console.error("[TodoPanel] Failed to update todo text", e);
    }
  }

  // Drag-and-drop handlers
  function onDragStart(e: DragEvent, id: string) {
    dragSourceId = id;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", id);
    }
  }

  function onDragOver(e: DragEvent, id: string) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dragOverId = id;
  }

  function onDragLeave() {
    dragOverId = null;
  }

  function onDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    dragOverId = null;
    if (!dragSourceId || dragSourceId === targetId) {
      dragSourceId = null;
      return;
    }

    // Build new ordered list: move dragSource before target
    const currentOrder = [...filtered].map((t) => t.id);
    const fromIdx = currentOrder.indexOf(dragSourceId);
    const toIdx = currentOrder.indexOf(targetId);
    if (fromIdx === -1 || toIdx === -1) { dragSourceId = null; return; }

    currentOrder.splice(fromIdx, 1);
    const insertAt = currentOrder.indexOf(targetId);
    currentOrder.splice(insertAt, 0, dragSourceId);

    dragSourceId = null;
    reorderTodos(currentOrder);
  }

  function onDragEnd() {
    dragSourceId = null;
    dragOverId = null;
  }

  onMount(() => {
    loadTodos();
  });
</script>

<div class="todo-panel">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">TODO</span>
    <span class="progress">{done}/{total}</span>
    <div class="filter-group">
      <button
        class="filter-btn"
        class:active={filter === "all"}
        onclick={() => { filter = "all"; }}
      >All</button>
      <button
        class="filter-btn"
        class:active={filter === "active"}
        onclick={() => { filter = "active"; }}
      >Active</button>
      <button
        class="filter-btn"
        class:active={filter === "done"}
        onclick={() => { filter = "done"; }}
      >Done</button>
    </div>
  </div>

  <!-- Add item input -->
  <div class="add-item-row">
    <input
      class="add-input"
      type="text"
      placeholder="Add task..."
      bind:value={newItemText}
      onkeydown={onNewItemKeydown}
    />
  </div>

  <!-- Todo list -->
  <div class="todo-list">
    {#if filtered.length === 0}
      <div class="empty-state">No tasks</div>
    {:else}
      {#each filtered as todo, index (todo.id)}
        <div
          class="todo-row"
          class:is-done={todo.is_done}
          class:drag-over={dragOverId === todo.id}
          draggable="true"
          ondragstart={(e) => onDragStart(e, todo.id)}
          ondragover={(e) => onDragOver(e, todo.id)}
          ondragleave={onDragLeave}
          ondrop={(e) => onDrop(e, todo.id)}
          ondragend={onDragEnd}
          role="listitem"
        >
          <!-- Drag handle -->
          <span class="drag-handle" aria-hidden="true">⠿</span>

          <!-- Checkbox -->
          <button
            class="checkbox-btn"
            aria-label={todo.is_done ? "Mark as active" : "Mark as done"}
            onclick={() => toggleTodo(todo.id)}
          >
            {#if todo.is_done}
              <span class="checkbox-checked">✓</span>
            {:else}
              <span class="checkbox-empty"></span>
            {/if}
          </button>

          <!-- Text -->
          <input
            class="todo-text-input"
            class:done-text={todo.is_done}
            type="text"
            value={todo.text}
            onkeydown={(e) => onItemKeydown(e, todo, index)}
            onblur={(e) => onItemBlur(todo, (e.currentTarget as HTMLInputElement).value)}
          />

          <!-- Delete -->
          <button
            class="delete-btn"
            aria-label="Delete task"
            onclick={() => deleteTodo(todo.id)}
          >✕</button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .todo-panel {
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
    gap: 8px;
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
  }

  .progress {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    flex: 1;
  }

  .filter-group {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    overflow: hidden;
  }

  .filter-btn {
    font-size: 11px;
    color: var(--text-secondary);
    background: none;
    border: none;
    border-right: 1px solid var(--border);
    padding: 2px 8px;
    cursor: pointer;
    font-family: var(--font-sans);
  }

  .filter-btn:last-child {
    border-right: none;
  }

  .filter-btn:hover {
    background: var(--bg-elevated);
    color: var(--text);
  }

  .filter-btn.active {
    background: var(--bg-elevated);
    color: var(--text);
    font-weight: 600;
  }

  /* Add item row */
  .add-item-row {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .add-input {
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: 13px;
    padding: 5px 8px;
    outline: none;
    box-sizing: border-box;
  }

  .add-input::placeholder {
    color: var(--text-tertiary);
  }

  .add-input:focus {
    border-color: var(--border-active);
  }

  /* Todo list */
  .todo-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    font-size: 11px;
    color: var(--text-tertiary);
    text-align: center;
    padding: 20px 0;
  }

  .todo-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    cursor: default;
    user-select: none;
  }

  .todo-row:hover {
    background: var(--bg-secondary);
  }

  .todo-row.drag-over {
    border-top: 2px solid var(--accent);
  }

  .drag-handle {
    font-size: 12px;
    color: var(--text-tertiary);
    cursor: grab;
    flex-shrink: 0;
    line-height: 1;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  /* Checkbox */
  .checkbox-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 16px;
    height: 16px;
  }

  .checkbox-empty {
    display: inline-block;
    width: 13px;
    height: 13px;
    border: 1px solid var(--border-active);
    border-radius: var(--radius-0);
  }

  .checkbox-checked {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 13px;
    height: 13px;
    background: var(--accent-success);
    border: 1px solid var(--accent-success);
    border-radius: var(--radius-0);
    color: var(--bg);
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
  }

  /* Todo text input */
  .todo-text-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: var(--font-sans);
    font-size: 13px;
    color: var(--text);
    padding: 0;
    min-width: 0;
  }

  .todo-text-input.done-text {
    text-decoration: line-through;
    color: var(--text-tertiary);
  }

  /* Delete button */
  .delete-btn {
    background: none;
    border: none;
    padding: 0 2px;
    cursor: pointer;
    font-size: 11px;
    color: var(--text-tertiary);
    opacity: 0;
    transition: opacity 0.1s;
    flex-shrink: 0;
    font-family: var(--font-sans);
  }

  .todo-row:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--text-error);
  }
</style>
