# Runyard | Agent Operating Procedures

Performance-first, agentic IDE for human-AI collaboration.

Runyard is envisioned as the powerful successor to VS Code, Cursor, and Windsurf, combining their agentic capabilities with phenomenal performance that surpasses Zed and other performance-focused editors.

---

## 🏗️ Project Structure
Runyard is a monorepo managed by `pnpm` and `turbo`.
- `apps/desktop`: Tauri v2 + Svelte 5 desktop application.
- `apps/mobile`: Tauri Mobile (shared core/UI).
- `apps/web`: Svelte 5 frontend served by sub-service.
- `crates/runyard-core`: Core Rust logic (FS, Git, Commands).
- `packages/ui`: Shared Svelte 5 component library (Runes).
- `packages/editor`: CodeMirror 6 configuration and wrapper.
- `packages/common`: Shared TypeScript types and utilities.
- `packages/protocol`: ACP/MCP/AgentCP protocol implementations.

---

## 🛠️ The Stack
- **Shell**: Tauri v2 (Rust)
- **Frontend**: Svelte 5 (Runes), TypeScript (Strict), TailwindCSS
- **Editor**: CodeMirror 6
- **Communication**: JSON-RPC over WebSocket (Sub-service) / IPC (Local)

---

## 📜 The Prime Directives
1. **Always Ask. Never Assume.** Confirm before destructive or ambiguous actions.
2. **Understand Before Acting.** Read existing code and schemas. No new libraries without approval.
3. **Finish What You Start.** No `TODO`, `FIXME`, or `...` placeholders.
4. **Performance is a Feature.** Every change must be evaluated for latency and memory impact. Aim for phenomenal performance.

---

## 💎 Best Practices & Patterns
- **Svelte 5**: Use `$state`, `$derived`, `$effect` exclusively. No legacy syntax.
- **Component Design**: Single responsibility. UI only; logic belongs in stores or modules.
- **State Management**: Use `$state` in `.svelte.ts` files for global/shared state.
- **Strict Types**: No `any`. Use `Zod` for all boundary validation (IPC, API).
- **Error Handling**: Explicit, informative errors. No swallowed exceptions. In Rust, no `unwrap()` in production.
- **Styling**: Tailwind utility-first. No inline styles. Use CSS variables for themes.
- **Performance**: Minimize IPC roundtrips. Optimize Svelte reactivity.

---

## ⚠️ Workflow & Safety
- **Git**: Always `git status` before big moves. PUSH to GitHub after every logical change.
- **Surgical Edits**: Prefer `replace` over `write_file` for large files. **Never** leave `...` placeholders in code.
- **Destructive Ops**: `rm -rf` and `.env` edits require explicit user confirmation.
- **Process Management**: Never kill all Node processes (you'll kill the agent).

> [!IMPORTANT]
> You are building a tool for agents. Your code is the blueprint. Make it exemplary.
