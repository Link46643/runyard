<script lang="ts">
  import type { ContentBlock, DiffBlock as DiffBlockType, ToolCallBlock as ToolCallBlockType, ToolResultBlock as ToolResultBlockType } from "@runyard/common";
  import TextBlockRenderer from "./TextBlock.svelte";
  import CodeBlockRenderer from "./CodeBlock.svelte";
  import DiffBlockRenderer from "./DiffBlock.svelte";
  import ToolCallBlockRenderer from "./ToolCallBlock.svelte";
  import ToolResultBlockRenderer from "./ToolResultBlock.svelte";
  import ThinkingBlockRenderer from "./ThinkingBlock.svelte";
  import PermissionBlockRenderer from "./PermissionBlock.svelte";
  import FileRefBlockRenderer from "./FileRefBlock.svelte";
  import PlanBlockRenderer from "./PlanBlock.svelte";
  import ContextSummaryBlockRenderer from "./ContextSummaryBlock.svelte";
  import ErrorBlockRenderer from "./ErrorBlock.svelte";

  let {
    content,
    onExplainCode,
    onOpenFile,
    onPermissionDecision,
  }: {
    content: ContentBlock[];
    onExplainCode?: (code: string, language: string) => void;
    onOpenFile?: (path: string) => void;
    onPermissionDecision?: (block: ContentBlock, approved: boolean, forSession?: boolean) => void;
  } = $props();

  function findResultFor(toolId: string): ToolResultBlockType | undefined {
    return content.find((b) => b.type === "tool_result" && b.tool_id === toolId) as ToolResultBlockType | undefined;
  }

  // Skip tool_result blocks that are already rendered nested under their tool_call.
  let visibleBlocks = $derived(
    content.filter((b) => {
      if (b.type !== "tool_result") return true;
      const hasMatchingCall = content.some((c) => c.type === "tool_call" && c.tool_id === b.tool_id);
      return !hasMatchingCall;
    })
  );

  // Group consecutive diff blocks into a single "Batch change" wrapper.
  type Group = { kind: "single"; block: ContentBlock } | { kind: "diff-batch"; blocks: DiffBlockType[] };
  let groups = $derived.by(() => {
    const result: Group[] = [];
    let i = 0;
    while (i < visibleBlocks.length) {
      const b = visibleBlocks[i];
      if (b.type === "diff") {
        const batch: DiffBlockType[] = [b];
        let j = i + 1;
        while (j < visibleBlocks.length && visibleBlocks[j].type === "diff") {
          batch.push(visibleBlocks[j] as DiffBlockType);
          j++;
        }
        if (batch.length > 1) {
          result.push({ kind: "diff-batch", blocks: batch });
        } else {
          result.push({ kind: "single", block: b });
        }
        i = j;
      } else {
        result.push({ kind: "single", block: b });
        i++;
      }
    }
    return result;
  });
</script>

<div class="block-stack">
  {#each groups as group, gi (gi)}
    {#if group.kind === "diff-batch"}
      <div class="batch-change">
        <div class="batch-header">Batch change - {group.blocks.length} files</div>
        <div class="batch-body">
          {#each group.blocks as block, bi (bi)}
            <DiffBlockRenderer {block} />
          {/each}
        </div>
      </div>
    {:else}
      {@const block = group.block}
      <div class="block-item">
        {#if block.type === "text"}
          <TextBlockRenderer {block} />
        {:else if block.type === "code"}
          <CodeBlockRenderer {block} onExplain={onExplainCode} />
        {:else if block.type === "diff"}
          <DiffBlockRenderer {block} />
        {:else if block.type === "tool_call"}
          <ToolCallBlockRenderer {block} result={findResultFor((block as ToolCallBlockType).tool_id)} />
        {:else if block.type === "tool_result"}
          <ToolResultBlockRenderer {block} />
        {:else if block.type === "thinking"}
          <ThinkingBlockRenderer {block} />
        {:else if block.type === "permission_request"}
          <PermissionBlockRenderer {block} onDecide={(approved, forSession) => onPermissionDecision?.(block, approved, forSession)} />
        {:else if block.type === "file_ref"}
          <FileRefBlockRenderer {block} onOpen={onOpenFile} />
        {:else if block.type === "plan"}
          <PlanBlockRenderer {block} />
        {:else if block.type === "context_summary"}
          <ContextSummaryBlockRenderer {block} />
        {:else if block.type === "error"}
          <ErrorBlockRenderer {block} />
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  .block-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .block-item {
    /* Per-block lazy rendering: off-screen blocks within a long message skip
       layout/paint work. */
    content-visibility: auto;
    contain-intrinsic-size: auto 80px;
  }
  .batch-change {
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }
  .batch-header {
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    padding: var(--space-2) var(--space-3);
  }
  .batch-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2);
  }
</style>
