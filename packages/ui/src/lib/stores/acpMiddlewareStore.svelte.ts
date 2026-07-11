// ACP conductor middleware (engineering-todo-v2.md 1.7.16).
//
// The agent-client-protocol-conductor crate is designed for building proxy/
// conductor ROLES, not plain IDE clients. Runyard's IDE is always a client,
// never a proxy. So the conductor crate itself is not used here.
//
// Instead, this module implements the four middleware capabilities listed in
// 1.7.16 directly against the ACP event stream that acpStore already exposes:
//
//   1. Request/response logging  → already covered by AcpEvent::LogLine (1.6.9)
//   2. Permission interception   → filter applied before respondPermission()
//   3. Tool filtering            → blocklist enforced at respond-permission time
//   4. Cost tracking             → accumulated from agent_task cost field
//
// This is the correct architecture for a client: intercept at the event layer,
// not at the protocol layer.

class AcpMiddlewareStore {
  // ── Tool filtering (1.7.16 "tool filtering") ─────────────────────────────
  // Tools on this list are auto-denied when the agent requests permission.
  // The user can manage this list in the settings panel.
  blockedTools = $state<string[]>([]);

  // ── Cost tracking (1.7.16 "cost tracking") ────────────────────────────────
  // Running total cost across all ACP sessions in this app session.
  // Populated from agent_tasks when available; updated from acp:session_info_update
  // events (agents that report cost data include it there).
  totalCostUsd = $state(0);
  sessionCosts = $state<Record<string, number>>({}); // sessionId -> cost

  // ── Permission interception (1.7.16 "permission interception") ────────────
  // Hooks registered by other components. Called before the permission card
  // is shown to the user. Return true to allow the permission request through
  // normally; return false to auto-deny.
  private _hooks: Array<(toolName: string, sessionId: string) => boolean> = [];

  registerPermissionHook(hook: (toolName: string, sessionId: string) => boolean): () => void {
    this._hooks.push(hook);
    return () => {
      this._hooks = this._hooks.filter((h) => h !== hook);
    };
  }

  /** Check whether a permission request should be shown or auto-denied. */
  shouldAllowPermission(toolName: string, sessionId: string): boolean {
    // Auto-deny if the tool is on the blocklist.
    if (this.blockedTools.includes(toolName)) return false;
    // Run all registered hooks; any false means deny.
    for (const hook of this._hooks) {
      if (!hook(toolName, sessionId)) return false;
    }
    return true;
  }

  // ── Tool blocklist management ─────────────────────────────────────────────
  blockTool(toolName: string) {
    if (!this.blockedTools.includes(toolName)) {
      this.blockedTools = [...this.blockedTools, toolName];
    }
  }

  unblockTool(toolName: string) {
    this.blockedTools = this.blockedTools.filter((t) => t !== toolName);
  }

  isBlocked(toolName: string): boolean {
    return this.blockedTools.includes(toolName);
  }

  // ── Cost accumulation ─────────────────────────────────────────────────────
  /** Called by the Tauri bridge / acpStore whenever a cost update arrives. */
  recordSessionCost(sessionId: string, costUsd: number) {
    const prev = this.sessionCosts[sessionId] ?? 0;
    const delta = Math.max(0, costUsd - prev); // agents report cumulative cost
    this.sessionCosts = { ...this.sessionCosts, [sessionId]: costUsd };
    this.totalCostUsd = this.totalCostUsd + delta;
  }

  resetCosts() {
    this.sessionCosts = {};
    this.totalCostUsd = 0;
  }
}

export const acpMiddlewareStore = new AcpMiddlewareStore();
