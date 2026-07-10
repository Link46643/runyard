<script lang="ts">
  // 1. Imports
  import { onMount, onDestroy } from "svelte";
  import {
    Plus,
    BookOpen,
    Pencil,
    Trash2,
    Search,
    RefreshCw,
  } from "lucide-svelte";
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { webSocketClient } from "@runyard/common";
  import type { SkillMetadata, SkillScope, ScannedSkillCandidate } from "@runyard/common";

  // 2. Types
  type DialogMode = "add" | "edit" | null;

  interface SkillTemplate {
    name: string;
    description: string;
    body: string;
  }

  const SKILL_TEMPLATES: SkillTemplate[] = [
    {
      name: "code-review",
      description: "Performs thorough code review with focus on correctness, style, and security",
      body: `Review the provided code for:
1. Logic errors and bugs
2. Security vulnerabilities (injection, XSS, auth issues)
3. Performance issues
4. Style and readability
5. Missing error handling

Provide specific, actionable feedback with line references where possible.`
    },
    {
      name: "debug",
      description: "Systematically debugs issues by analyzing error messages and tracing execution",
      body: `Debug the reported issue:
1. Identify the error type and message
2. Trace the execution path to the failure point
3. Check for common causes (null refs, type mismatches, off-by-one)
4. Propose a fix with explanation
5. Suggest how to prevent similar issues`
    },
    {
      name: "refactor",
      description: "Refactors code for clarity, maintainability, and following project conventions",
      body: `Refactor the code:
1. Identify code smells (long functions, deep nesting, duplicates)
2. Apply appropriate patterns (extract function, rename, simplify)
3. Preserve existing behavior - include tests if available
4. Follow project conventions from surrounding code`
    },
    {
      name: "test-generation",
      description: "Generates comprehensive unit and integration tests for the given code",
      body: `Generate tests:
1. Identify all public interfaces and functions
2. Cover happy paths, edge cases, and error conditions
3. Use the project's existing test framework and conventions
4. Aim for >80% branch coverage
5. Include both unit tests and integration tests where appropriate`
    },
    {
      name: "documentation",
      description: "Writes clear documentation: docstrings, README updates, inline comments",
      body: `Write documentation:
1. Add docstrings/JSDoc to all public functions and types
2. Update README if behavior changes
3. Add inline comments for non-obvious logic only
4. Keep explanations concise and accurate`
    },
    {
      name: "api-migration",
      description: "Migrates code from one API version to another safely",
      body: `Migrate the API usage:
1. Identify all usage sites of the old API
2. Understand breaking changes in the new version
3. Update each usage site, preserving semantics
4. Add compatibility shims if needed for gradual migration
5. Update types and interfaces`
    },
    {
      name: "security-audit",
      description: "Audits code for security vulnerabilities and compliance issues",
      body: `Security audit:
1. Check for injection vulnerabilities (SQL, command, template)
2. Verify authentication and authorization
3. Check for sensitive data exposure
4. Review dependency security
5. Check for insecure cryptography
Report severity: Critical / High / Medium / Low`
    },
    {
      name: "performance-optimize",
      description: "Identifies and fixes performance bottlenecks",
      body: `Optimize performance:
1. Profile the hot path (identify the bottleneck)
2. Check for N+1 queries, unnecessary loops, memory leaks
3. Consider caching opportunities
4. Measure before and after
5. Document the trade-offs of each change`
    },
    {
      name: "deployment",
      description: "Assists with deployment preparation: CI/CD, environment config, health checks",
      body: `Deployment preparation:
1. Review environment variables and secrets management
2. Check health check endpoints
3. Review CI/CD pipeline steps
4. Verify rollback strategy
5. Check logging and monitoring setup`
    },
  ];

  // Dual-mode invoke (Tauri or WebSocket)
  async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  // 3. Props (none — top-level panel)

  // 4. State
  let skills = $state<SkillMetadata[]>([]);
  let isLoading = $state(false);
  let error = $state<string | null>(null);

  let dialogMode = $state<DialogMode>(null);
  let editingSkill = $state<SkillMetadata | null>(null);

  // Form fields
  let formName = $state("");
  let formDescription = $state("");
  let formScope = $state<SkillScope>("global");
  let formWhenToUse = $state("");
  let formBody = $state("");

  // Form validation errors
  let formErrors = $state<{ name?: string; body?: string; description?: string }>({});

  // Delete confirmation
  let deleteConfirmId = $state<string | null>(null);

  // Hover state for @mention chip
  let hoveredSkillId = $state<string | null>(null);

  // Scan dialog
  let showScanDialog = $state(false);
  let scanLoading = $state(false);
  let scanError = $state<string | null>(null);
  let scannedCandidates = $state<ScannedSkillCandidate[]>([]);

  // Body textarea ref for auto-resize
  let bodyTextarea = $state<HTMLTextAreaElement | null>(null);

  // File watcher unlisten function
  let _fsWatchUnlisten: (() => void) | null = null;

  // Selected template (empty string = blank)
  let selectedTemplate = $state<string>("");

  // 5. Derived
  let globalSkills = $derived(skills.filter((s) => s.scope === "global"));
  let projectSkills = $derived(skills.filter((s) => s.scope === "project"));
  let nestedSkills = $derived(skills.filter((s) => s.scope === "nested"));

  let bodyCharCount = $derived(formBody.length);
  let bodyLineCount = $derived(formBody.split("\n").length);

  // Inline live validation (separate from submit-time validateForm)
  let nameInlineError = $derived.by(() => {
    const slug = formName.trim();
    if (!slug) return null;
    if (slug.length > 64) return "Name must be lowercase letters, digits, and hyphens only. Max 64 chars.";
    if (!/^[a-z][a-z0-9-]*$/.test(slug)) return "Name must be lowercase letters, digits, and hyphens only. Max 64 chars.";
    return null;
  });

  let bodyLineWarning = $derived(
    bodyLineCount > 500 ? "Body exceeds 500 lines. Trim to reduce agent context usage." : null
  );

  let canSubmit = $derived(
    formName.trim().length > 0 &&
    formBody.trim().length > 0 &&
    formDescription.trim().length > 0 &&
    nameInlineError === null &&
    Object.keys(formErrors).length === 0
  );

  // 6. Effects
  $effect(() => {
    // Auto-resize body textarea
    if (bodyTextarea) {
      bodyTextarea.style.height = "auto";
      bodyTextarea.style.height = `${bodyTextarea.scrollHeight}px`;
    }
  });

  // 7. Functions

  // ── Load ──────────────────────────────────────────────────────────────────
  async function loadSkills() {
    isLoading = true;
    error = null;
    try {
      skills = await invoke<SkillMetadata[]>("skill_list");
    } catch (e) {
      error = String(e);
    } finally {
      isLoading = false;
    }
  }

  // ── Toggle active ─────────────────────────────────────────────────────────
  async function toggleActive(skill: SkillMetadata) {
    error = null;
    try {
      const updated = await invoke<SkillMetadata>("skill_update", {
        id: skill.id,
        name: skill.name,
        description: skill.description,
        body: skill.body,
        whenToUse: skill.when_to_use,
        isActive: !skill.is_active,
      });
      skills = skills.map((s) => (s.id === skill.id ? updated : s));
    } catch (e) {
      error = String(e);
    }
  }

  // ── Delete ────────────────────────────────────────────────────────────────
  function openDeleteConfirm(id: string) {
    deleteConfirmId = id;
  }

  function closeDeleteConfirm() {
    deleteConfirmId = null;
  }

  async function confirmDelete(skill: SkillMetadata) {
    error = null;
    try {
      await invoke("skill_delete", { id: skill.id });
      skills = skills.filter((s) => s.id !== skill.id);
    } catch (e) {
      error = String(e);
    } finally {
      deleteConfirmId = null;
    }
  }

  // ── Validation ────────────────────────────────────────────────────────────
  function validateForm(): boolean {
    const errs: typeof formErrors = {};
    const slug = formName.trim();
    if (!slug) {
      errs.name = "Name is required.";
    } else if (slug.length > 64 || !/^[a-z][a-z0-9-]*$/.test(slug)) {
      errs.name = "Name must be lowercase letters, digits, and hyphens only. Max 64 chars.";
    }
    if (!formDescription.trim()) {
      errs.description = "Description is required.";
    }
    if (!formBody.trim()) {
      errs.body = "Body is required.";
    }
    formErrors = errs;
    return Object.keys(errs).length === 0;
  }

  // ── Dialog open / close ───────────────────────────────────────────────────
  function openAddDialog(prefill?: Partial<{ name: string; description: string; body: string; scope: SkillScope }>) {
    dialogMode = "add";
    editingSkill = null;
    resetForm();
    if (prefill) {
      if (prefill.name) formName = prefill.name;
      if (prefill.description) formDescription = prefill.description;
      if (prefill.body) formBody = prefill.body;
      if (prefill.scope) formScope = prefill.scope;
    }
  }

  function openEditDialog(skill: SkillMetadata) {
    dialogMode = "edit";
    editingSkill = skill;
    formName = skill.name;
    formDescription = skill.description;
    formScope = skill.scope;
    formWhenToUse = skill.when_to_use ?? "";
    formBody = skill.body;
    formErrors = {};
  }

  function closeDialog() {
    dialogMode = null;
    editingSkill = null;
    resetForm();
  }

  function resetForm() {
    formName = "";
    formDescription = "";
    formScope = "global";
    formWhenToUse = "";
    formBody = "";
    formErrors = {};
    selectedTemplate = "";
  }

  // ── Template selection ────────────────────────────────────────────────────
  function applyTemplate(templateName: string) {
    if (!templateName) return;
    const tpl = SKILL_TEMPLATES.find((t) => t.name === templateName);
    if (!tpl) return;
    formName = tpl.name;
    formDescription = tpl.description;
    formBody = tpl.body;
    formScope = "project";
    formErrors = {};
  }

  // ── Submit ────────────────────────────────────────────────────────────────
  async function submitForm() {
    if (!validateForm()) return;
    error = null;

    try {
      if (dialogMode === "edit" && editingSkill) {
        const updated = await invoke<SkillMetadata>("skill_update", {
          id: editingSkill.id,
          name: formName.trim(),
          description: formDescription.trim(),
          body: formBody,
          whenToUse: formWhenToUse.trim() || null,
          isActive: editingSkill.is_active,
        });
        skills = skills.map((s) => (s.id === editingSkill!.id ? updated : s));
      } else {
        const created = await invoke<SkillMetadata>("skill_create", {
          name: formName.trim(),
          description: formDescription.trim(),
          scope: formScope,
          directoryPath: "",
          filePath: "",
          body: formBody,
          whenToUse: formWhenToUse.trim() || null,
        });
        skills = [...skills, created];
      }
      closeDialog();
    } catch (e) {
      error = String(e);
    }
  }

  // ── Scan ──────────────────────────────────────────────────────────────────
  function openScanDialog() {
    showScanDialog = true;
    scannedCandidates = [];
    scanError = null;
    runScan();
  }

  function closeScanDialog() {
    showScanDialog = false;
    scannedCandidates = [];
    scanError = null;
  }

  async function runScan() {
    scanLoading = true;
    scanError = null;
    try {
      scannedCandidates = await invoke<ScannedSkillCandidate[]>("skill_scan_directories", {
        workspacePath: "../../",
      });
    } catch (e) {
      scanError = String(e);
    } finally {
      scanLoading = false;
    }
  }

  function importScannedSkill(candidate: ScannedSkillCandidate) {
    closeScanDialog();
    openAddDialog({
      name: candidate.name.toLowerCase().replace(/\s+/g, "-"),
      description: candidate.preview.split("\n")[0].slice(0, 120),
      body: candidate.preview,
      scope: "project",
    });
  }

  // 8. Lifecycle
  onMount(async () => {
    loadSkills();

    // Set up file watcher for the skills directory (best-effort)
    try {
      await invoke("fs_watch", { path: "../../.claude/skills" });
      const unlisten = await listen("fs:changed", () => {
        loadSkills();
      });
      _fsWatchUnlisten = unlisten;
    } catch {
      // Silently ignore — directory may not exist
    }
  });

  onDestroy(() => {
    if (_fsWatchUnlisten) {
      _fsWatchUnlisten();
      _fsWatchUnlisten = null;
    }
  });
</script>

<div class="skills-panel">
  <!-- Panel header -->
  <div class="panel-header">
    <span class="panel-title">SKILLS</span>
    <div class="header-actions">
      <button class="btn-ghost btn-sm" onclick={openScanDialog} title="Scan for skills">
        <Search size={12} />
        Scan
      </button>
      <button class="btn-primary btn-sm" onclick={() => openAddDialog()}>
        <Plus size={12} />
        New skill
      </button>
    </div>
  </div>

  <!-- Global error banner -->
  {#if error}
    <div class="global-error">{error}</div>
  {/if}

  <!-- Skill list -->
  <div class="skills-list">
    {#if isLoading}
      <div class="empty-state">
        <RefreshCw size={14} color="var(--text-tertiary)" />
        <span>Loading skills...</span>
      </div>
    {:else if skills.length === 0}
      <div class="empty-state">
        <BookOpen size={48} color="var(--text-tertiary)" />
        <p>No skills yet.<br />Scan for installed skills or create one.</p>
        <div class="empty-actions">
          <button class="btn-primary btn-sm" onclick={() => openAddDialog()}>
            <Plus size={12} />
            New skill
          </button>
          <button class="btn-ghost btn-sm" onclick={openScanDialog}>
            <Search size={12} />
            Scan
          </button>
        </div>
      </div>
    {:else}
      <!-- Global skills group -->
      {#if globalSkills.length > 0}
        <div class="skill-group">
          <div class="group-header">Global</div>
          {#each globalSkills as skill (skill.id)}
            {@const isHovered = hoveredSkillId === skill.id}
            <div
              class="skill-row-wrapper"
              onmouseenter={() => { hoveredSkillId = skill.id; }}
              onmouseleave={() => { hoveredSkillId = null; }}
              role="group"
            >
              <div class="skill-row">
                <div class="skill-main">
                  <span
                    class="status-dot"
                    style="background:{skill.is_active ? 'var(--text-success)' : 'var(--text-tertiary)'}"
                  ></span>
                  <div class="skill-info">
                    <div class="skill-name-line">
                      <span class="skill-name">{skill.name}</span>
                      {#if isHovered}
                        <span class="mention-chip">@skill:{skill.name}</span>
                      {/if}
                      {#if skill.is_builtin}
                        <span class="badge builtin-badge">built-in</span>
                      {/if}
                    </div>
                    {#if skill.description}
                      <span class="skill-desc">{skill.description}</span>
                    {/if}
                  </div>
                </div>

                <div class="skill-actions">
                  <button
                    class="btn-ghost btn-sm"
                    onclick={() => toggleActive(skill)}
                  >
                    {skill.is_active ? "Disable" : "Enable"}
                  </button>
                  <button
                    class="btn-icon"
                    title="Edit skill"
                    onclick={() => openEditDialog(skill)}
                    disabled={skill.is_builtin}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    class="btn-icon btn-icon-danger"
                    title="Delete skill"
                    onclick={() => openDeleteConfirm(skill.id)}
                    disabled={skill.is_builtin}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>

              {#if deleteConfirmId === skill.id}
                <div class="delete-confirm">
                  <span class="delete-msg">
                    Delete <strong>{skill.name}</strong>? This cannot be undone.
                  </span>
                  <div class="delete-actions">
                    <button class="btn-danger btn-sm" onclick={() => confirmDelete(skill)}>
                      Delete
                    </button>
                    <button class="btn-ghost btn-sm" onclick={closeDeleteConfirm}>
                      Cancel
                    </button>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Project skills group -->
      {#if projectSkills.length > 0}
        <div class="skill-group">
          <div class="group-header">Project</div>
          {#each projectSkills as skill (skill.id)}
            {@const isHovered = hoveredSkillId === skill.id}
            <div
              class="skill-row-wrapper"
              onmouseenter={() => { hoveredSkillId = skill.id; }}
              onmouseleave={() => { hoveredSkillId = null; }}
              role="group"
            >
              <div class="skill-row">
                <div class="skill-main">
                  <span
                    class="status-dot"
                    style="background:{skill.is_active ? 'var(--text-success)' : 'var(--text-tertiary)'}"
                  ></span>
                  <div class="skill-info">
                    <div class="skill-name-line">
                      <span class="skill-name">{skill.name}</span>
                      {#if isHovered}
                        <span class="mention-chip">@skill:{skill.name}</span>
                      {/if}
                    </div>
                    {#if skill.description}
                      <span class="skill-desc">{skill.description}</span>
                    {/if}
                  </div>
                </div>

                <div class="skill-actions">
                  <button
                    class="btn-ghost btn-sm"
                    onclick={() => toggleActive(skill)}
                  >
                    {skill.is_active ? "Disable" : "Enable"}
                  </button>
                  <button
                    class="btn-icon"
                    title="Edit skill"
                    onclick={() => openEditDialog(skill)}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    class="btn-icon btn-icon-danger"
                    title="Delete skill"
                    onclick={() => openDeleteConfirm(skill.id)}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>

              {#if deleteConfirmId === skill.id}
                <div class="delete-confirm">
                  <span class="delete-msg">
                    Delete <strong>{skill.name}</strong>? This cannot be undone.
                  </span>
                  <div class="delete-actions">
                    <button class="btn-danger btn-sm" onclick={() => confirmDelete(skill)}>
                      Delete
                    </button>
                    <button class="btn-ghost btn-sm" onclick={closeDeleteConfirm}>
                      Cancel
                    </button>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}

      <!-- Nested skills group -->
      {#if nestedSkills.length > 0}
        <div class="skill-group">
          <div class="group-header">Nested</div>
          {#each nestedSkills as skill (skill.id)}
            {@const isHovered = hoveredSkillId === skill.id}
            <div
              class="skill-row-wrapper"
              onmouseenter={() => { hoveredSkillId = skill.id; }}
              onmouseleave={() => { hoveredSkillId = null; }}
              role="group"
            >
              <div class="skill-row">
                <div class="skill-main">
                  <span
                    class="status-dot"
                    style="background:{skill.is_active ? 'var(--text-success)' : 'var(--text-tertiary)'}"
                  ></span>
                  <div class="skill-info">
                    <div class="skill-name-line">
                      <span class="skill-name">{skill.name}</span>
                      {#if isHovered}
                        <span class="mention-chip">@skill:{skill.name}</span>
                      {/if}
                    </div>
                    {#if skill.description}
                      <span class="skill-desc">{skill.description}</span>
                    {/if}
                  </div>
                </div>

                <div class="skill-actions">
                  <button
                    class="btn-ghost btn-sm"
                    onclick={() => toggleActive(skill)}
                  >
                    {skill.is_active ? "Disable" : "Enable"}
                  </button>
                  <button
                    class="btn-icon"
                    title="Edit skill"
                    onclick={() => openEditDialog(skill)}
                  >
                    <Pencil size={12} />
                  </button>
                  <button
                    class="btn-icon btn-icon-danger"
                    title="Delete skill"
                    onclick={() => openDeleteConfirm(skill.id)}
                  >
                    <Trash2 size={12} />
                  </button>
                </div>
              </div>

              {#if deleteConfirmId === skill.id}
                <div class="delete-confirm">
                  <span class="delete-msg">
                    Delete <strong>{skill.name}</strong>? This cannot be undone.
                  </span>
                  <div class="delete-actions">
                    <button class="btn-danger btn-sm" onclick={() => confirmDelete(skill)}>
                      Delete
                    </button>
                    <button class="btn-ghost btn-sm" onclick={closeDeleteConfirm}>
                      Cancel
                    </button>
                  </div>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

  <!-- @skill mention hint -->
  {#if skills.length > 0}
    <div class="mention-hint">
      Type <code>@skill:name</code> in chat to use a skill
    </div>
  {/if}
</div>

<!-- New / Edit skill dialog -->
{#if dialogMode !== null}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) closeDialog(); }}>
    <div class="modal" role="dialog" aria-modal="true">
      <div class="modal-header">
        <h2 class="modal-title">
          {dialogMode === "edit" ? "Edit skill" : "New skill"}
        </h2>
        <button class="btn-icon" onclick={closeDialog} aria-label="Close">✕</button>
      </div>

      <form class="skill-form" onsubmit={(e) => { e.preventDefault(); submitForm(); }}>
        <!-- Template selector (add mode only) -->
        {#if dialogMode === "add"}
          <div class="form-row">
            <label class="form-label" for="skill-template">Template</label>
            <select
              id="skill-template"
              class="form-select"
              bind:value={selectedTemplate}
              onchange={() => applyTemplate(selectedTemplate)}
            >
              <option value="">— Blank —</option>
              {#each SKILL_TEMPLATES as tpl (tpl.name)}
                <option value={tpl.name}>{tpl.name}</option>
              {/each}
            </select>
          </div>
        {/if}

        <!-- Name -->
        <div class="form-row">
          <label class="form-label" for="skill-name">Name</label>
          <input
            id="skill-name"
            class="form-input"
            class:input-error={!!formErrors.name || !!nameInlineError}
            type="text"
            placeholder="my-skill-name"
            bind:value={formName}
            required
          />
          <span class="form-hint">Lowercase letters, numbers, and hyphens only. Max 64 chars.</span>
          {#if nameInlineError}
            <span class="field-error">{nameInlineError}</span>
          {:else if formErrors.name}
            <span class="field-error">{formErrors.name}</span>
          {/if}
        </div>

        <!-- Description -->
        <div class="form-row">
          <label class="form-label" for="skill-desc">Description</label>
          <textarea
            id="skill-desc"
            class="form-textarea"
            class:input-error={!!formErrors.description}
            rows={2}
            placeholder="Short description of what this skill does."
            bind:value={formDescription}
          ></textarea>
          {#if formErrors.description}
            <span class="field-error">{formErrors.description}</span>
          {/if}
        </div>

        <!-- Scope -->
        {#if dialogMode === "add"}
          <div class="form-row">
            <label class="form-label" for="skill-scope">Scope</label>
            <select id="skill-scope" class="form-select" bind:value={formScope}>
              <option value="global">Global</option>
              <option value="project">Project</option>
              <option value="nested">Nested</option>
            </select>
          </div>
        {/if}

        <!-- When to use -->
        <div class="form-row">
          <label class="form-label" for="skill-when">When to use <span class="optional-tag">(optional)</span></label>
          <textarea
            id="skill-when"
            class="form-textarea"
            rows={2}
            placeholder="Describe when the AI should use this skill."
            bind:value={formWhenToUse}
          ></textarea>
        </div>

        <!-- Body -->
        <div class="form-row">
          <div class="body-label-row">
            <label class="form-label" for="skill-body">Body (Markdown)</label>
            <span class="char-counter" class:char-over={bodyCharCount > 25000}>
              {bodyCharCount.toLocaleString()} / 25,000 chars
            </span>
          </div>
          <textarea
            id="skill-body"
            class="form-textarea body-textarea"
            class:input-error={!!formErrors.body}
            rows={8}
            placeholder="Write the skill instructions in Markdown..."
            bind:value={formBody}
            bind:this={bodyTextarea}
            required
          ></textarea>
          {#if bodyLineWarning}
            <span class="field-warning">{bodyLineWarning}</span>
          {/if}
          {#if formErrors.body}
            <span class="field-error">{formErrors.body}</span>
          {/if}
        </div>

        <div class="form-footer">
          <button type="button" class="btn-ghost" onclick={closeDialog}>Cancel</button>
          <button type="submit" class="btn-primary" disabled={!canSubmit}>
            {dialogMode === "edit" ? "Save changes" : "Create skill"}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Scan dialog -->
{#if showScanDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) closeScanDialog(); }}>
    <div class="modal" role="dialog" aria-modal="true">
      <div class="modal-header">
        <h2 class="modal-title">Scan for skills</h2>
        <button class="btn-icon" onclick={closeScanDialog} aria-label="Close">✕</button>
      </div>

      <div class="scan-content">
        {#if scanLoading}
          <div class="scan-loading">
            <RefreshCw size={14} color="var(--text-secondary)" />
            <span>Scanning directories...</span>
          </div>
        {:else if scanError}
          <div class="scan-error">{scanError}</div>
          <button class="btn-ghost btn-sm" onclick={runScan}>Retry</button>
        {:else if scannedCandidates.length === 0}
          <div class="scan-empty">
            <BookOpen size={32} color="var(--text-tertiary)" />
            <span>No SKILL.md files found.</span>
            <button class="btn-ghost btn-sm" onclick={runScan}>Scan again</button>
          </div>
        {:else}
          <div class="scan-results">
            {#each scannedCandidates as candidate}
              <div class="scan-card">
                <div class="scan-card-info">
                  <span class="scan-card-name">{candidate.name}</span>
                  <span class="scan-card-path">{candidate.file_path}</span>
                  <p class="scan-card-preview">{candidate.preview.slice(0, 200)}{candidate.preview.length > 200 ? "…" : ""}</p>
                </div>
                <button
                  class="btn-primary btn-sm"
                  onclick={() => importScannedSkill(candidate)}
                >
                  Import
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div class="modal-footer">
        <button class="btn-ghost" onclick={closeScanDialog}>Close</button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Root */
  .skills-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    font-size: 12px;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
  }

  /* Panel header */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Global error */
  .global-error {
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text-error);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-error);
    flex-shrink: 0;
  }

  /* Skills list */
  .skills-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  /* Empty state */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 16px;
    color: var(--text-tertiary);
    font-size: 12px;
    text-align: center;
  }

  .empty-state p {
    margin: 0;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .empty-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: center;
  }

  /* Group headers */
  .skill-group {
    margin-bottom: 4px;
  }

  .group-header {
    padding: 6px 10px 3px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    border-bottom: 1px solid var(--border-secondary);
  }

  /* Skill rows */
  .skill-row-wrapper {
    border-bottom: 1px solid var(--border-secondary);
  }

  .skill-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
  }

  .skill-row:hover {
    background: var(--bg-secondary);
  }

  .skill-main {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    flex-shrink: 0;
    margin-top: 4px;
  }

  .skill-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .skill-name-line {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .skill-name {
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
  }

  .skill-desc {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* @mention chip — shown on hover */
  .mention-chip {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--text-link);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    border-radius: var(--radius-1);
    padding: 1px 5px;
    white-space: nowrap;
  }

  .skill-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  /* Badges */
  .badge {
    background: var(--bg-tertiary);
    border-radius: var(--radius-1);
    font-size: 10px;
    font-weight: 500;
    padding: 1px 5px;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .builtin-badge {
    color: var(--text-tertiary);
  }

  /* Delete confirmation */
  .delete-confirm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px 8px 24px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-secondary);
  }

  .delete-msg {
    font-size: 11px;
    color: var(--text-secondary);
    flex: 1;
  }

  .delete-msg strong {
    color: var(--text);
    font-weight: 500;
  }

  .delete-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  /* @mention hint callout */
  .mention-hint {
    flex-shrink: 0;
    padding: 7px 10px;
    font-size: 11px;
    color: var(--text-tertiary);
    border-top: 1px solid var(--border-secondary);
    background: var(--bg-secondary);
  }

  .mention-hint code {
    font-family: var(--font-mono);
    font-size: 10px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-1);
    padding: 1px 4px;
    color: var(--text-secondary);
  }

  /* Buttons */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent);
    color: var(--bg);
    border: none;
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-ghost:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .btn-danger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent-danger);
    color: var(--bg);
    border: none;
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-danger:hover {
    filter: brightness(1.1);
  }

  .btn-sm {
    padding: 3px 8px;
    font-size: 11px;
  }

  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    border-radius: var(--radius-1);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
  }

  .btn-icon:hover:not(:disabled) {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .btn-icon:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .btn-icon-danger:hover:not(:disabled) {
    color: var(--accent-danger);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-elevated);
    border-radius: var(--radius-3);
    box-shadow: var(--shadow-2);
    padding: 24px;
    width: 100%;
    max-width: 560px;
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid var(--border);
    padding-top: 8px;
  }

  /* Forms */
  .skill-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .optional-tag {
    font-weight: 400;
    text-transform: none;
    color: var(--text-tertiary);
    letter-spacing: 0;
    font-size: 11px;
  }

  .form-hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .form-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--border-active);
  }

  .form-input.input-error {
    border-color: var(--border-error);
  }

  .form-textarea {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    min-height: 40px;
  }

  .form-textarea:focus {
    outline: none;
    border-color: var(--border-active);
  }

  .form-textarea.input-error {
    border-color: var(--border-error);
  }

  .body-textarea {
    font-family: var(--font-mono);
    font-size: 12px;
    min-height: 160px;
    resize: vertical;
  }

  .form-select {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
    cursor: pointer;
  }

  .form-select:focus {
    outline: none;
    border-color: var(--border-active);
  }

  .field-error {
    font-size: 11px;
    color: var(--text-error);
  }

  .field-warning {
    font-size: 11px;
    color: var(--text-error);
  }

  .body-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .char-counter {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .char-counter.char-over {
    color: var(--text-error);
  }

  /* Form footer */
  .form-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
  }

  /* Scan dialog */
  .scan-content {
    min-height: 120px;
  }

  .scan-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 16px 0;
  }

  .scan-error {
    color: var(--text-error);
    font-size: 12px;
    padding: 8px 0;
  }

  .scan-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 24px 0;
    color: var(--text-tertiary);
    font-size: 12px;
    text-align: center;
  }

  .scan-results {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 50vh;
    overflow-y: auto;
  }

  .scan-card {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2);
    background: var(--bg-secondary);
  }

  .scan-card-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .scan-card-name {
    font-weight: 500;
    color: var(--text);
    font-size: 12px;
  }

  .scan-card-path {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .scan-card-preview {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
