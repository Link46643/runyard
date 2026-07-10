<script lang="ts">
  import "katex/dist/katex.min.css";
  import type { TextBlock } from "@runyard/common";
  import { renderMarkdownLite } from "./markdownLite.js";
  import Lightbox from "./Lightbox.svelte";

  let { block }: { block: TextBlock } = $props();
  let html = $derived(renderMarkdownLite(block.text));
  let container: HTMLDivElement;
  let lightboxSrc = $state<string | null>(null);

  async function renderMermaidBlocks() {
    if (!container) return;
    const pending = container.querySelectorAll<HTMLDivElement>(".mermaid-pending");
    if (pending.length === 0) return;
    const mermaid = (await import("mermaid")).default;
    mermaid.initialize({ startOnLoad: false, securityLevel: "strict" });
    for (const el of Array.from(pending)) {
      const source = el.textContent ?? "";
      try {
        const id = `mermaid-${Math.random().toString(36).slice(2)}`;
        const { svg } = await mermaid.render(id, source);
        el.innerHTML = svg;
        el.classList.remove("mermaid-pending");
        el.classList.add("mermaid-rendered");
      } catch (e) {
        el.textContent = "Diagram failed to render.";
        el.classList.add("mermaid-error");
      }
    }
  }

  function handleContainerClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.tagName === "IMG" && target.dataset.lightbox) {
      lightboxSrc = target.dataset.lightbox;
    }
  }

  $effect(() => {
    void html; // re-run after markup (re)renders
    renderMermaidBlocks();
  });
</script>

<div class="text-block" bind:this={container} onclick={handleContainerClick} role="presentation">
  {@html html}
</div>

<Lightbox src={lightboxSrc} onClose={() => (lightboxSrc = null)} />

<style>
  .text-block {
    font-family: var(--font-sans);
    font-size: var(--text-md);
    color: var(--text);
    line-height: 1.6;
  }
  .text-block :global(p) {
    margin: 0 0 var(--space-3) 0;
  }
  .text-block :global(p:last-child) {
    margin-bottom: 0;
  }
  .text-block :global(h1),
  .text-block :global(h2),
  .text-block :global(h3),
  .text-block :global(h4),
  .text-block :global(h5),
  .text-block :global(h6) {
    font-weight: 600;
    margin: var(--space-4) 0 var(--space-2) 0;
  }
  .text-block :global(a) {
    color: var(--text-link);
    text-decoration: none;
  }
  .text-block :global(a:hover) {
    text-decoration: underline;
  }
  .text-block :global(code) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border-radius: var(--radius-1);
  }
  .text-block :global(pre.md-code) {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    background: var(--editor-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    padding: var(--space-4);
    overflow-x: auto;
    margin: var(--space-3) 0;
  }
  .text-block :global(pre.md-code code) {
    background: none;
    padding: 0;
  }
  .text-block :global(ul),
  .text-block :global(ol) {
    margin: 0 0 var(--space-3) 0;
    padding-left: var(--space-6);
  }
  .text-block :global(li) {
    margin-bottom: var(--space-1);
  }
  .text-block :global(blockquote) {
    border-left: 2px solid var(--border);
    padding-left: var(--space-4);
    color: var(--text-secondary);
    margin: 0 0 var(--space-3) 0;
  }
  .text-block :global(table) {
    border-collapse: collapse;
    margin: var(--space-3) 0;
    font-size: var(--text-base);
  }
  .text-block :global(th),
  .text-block :global(td) {
    border: 1px solid var(--border);
    padding: var(--space-2) var(--space-3);
    text-align: left;
  }
  .text-block :global(th) {
    background: var(--bg-tertiary);
    font-weight: 600;
  }
  .text-block :global(.md-image) {
    max-width: 100%;
    border-radius: var(--radius-2);
    cursor: zoom-in;
    margin: var(--space-2) 0;
    display: block;
  }
  .text-block :global(.md-math-block) {
    margin: var(--space-3) 0;
    overflow-x: auto;
  }
  .text-block :global(.mermaid-pending) {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
    padding: var(--space-3);
    border-radius: var(--radius-0);
    white-space: pre;
  }
  .text-block :global(.mermaid-rendered) {
    display: flex;
    justify-content: center;
    margin: var(--space-3) 0;
    overflow-x: auto;
  }
  .text-block :global(.mermaid-error) {
    color: var(--text-error);
    font-size: var(--text-sm);
  }
</style>
