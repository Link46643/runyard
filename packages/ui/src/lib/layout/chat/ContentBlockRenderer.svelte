<script lang="ts">
  import type { ContentBlock, ToolCallBlock as ToolCallBlockType, ToolResultBlock as ToolResultBlockType } from "@runyard/common";
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
    onPermissionDecision?: (block: ContentBlock, approved: boolean) => void;
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
</script>

<div class="block-stack">
  {#each visibleBlocks as block, i (i)}
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
      <PermissionBlockRenderer {block} onDecide={(approved) => onPermissionDecision?.(block, approved)} />
    {:else if block.type === "file_ref"}
      <FileRefBlockRenderer {block} onOpen={onOpenFile} />
    {:else if block.type === "plan"}
      <PlanBlockRenderer {block} />
    {:else if block.type === "context_summary"}
      <ContextSummaryBlockRenderer {block} />
    {:else if block.type === "error"}
      <ErrorBlockRenderer {block} />
    {/if}
  {/each}
</div>

<style>
  .block-stack {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
</style>
