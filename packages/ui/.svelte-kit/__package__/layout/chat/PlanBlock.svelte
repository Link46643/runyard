<script lang="ts">
  import type { PlanBlock } from "@runyard/common";
  import { Circle, CircleDot, CircleCheck, CircleX } from "lucide-svelte";

  let { block }: { block: PlanBlock } = $props();

  let completedCount = $derived(block.steps.filter((s) => s.status === "completed").length);
  let total = $derived(block.steps.length);
</script>

<div class="plan-block">
  <div class="plan-header">
    <span>Plan</span>
    <span class="plan-progress">{completedCount}/{total}</span>
  </div>
  <div class="plan-progress-bar">
    <div class="plan-progress-fill" style:width={`${total > 0 ? (completedCount / total) * 100 : 0}%`}></div>
  </div>
  <ol class="plan-steps">
    {#each block.steps as step (step.id)}
      <li class="plan-step" class:failed={step.status === "failed"}>
        {#if step.status === "completed"}
          <CircleCheck size={14} strokeWidth={1.5} class="icon-success" />
        {:else if step.status === "running"}
          <CircleDot size={14} strokeWidth={1.5} class="icon-running" />
        {:else if step.status === "failed"}
          <CircleX size={14} strokeWidth={1.5} class="icon-error" />
        {:else}
          <Circle size={14} strokeWidth={1.5} class="icon-pending" />
        {/if}
        <span>{step.description}</span>
      </li>
    {/each}
  </ol>
</div>

<style>
  .plan-block {
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    padding: var(--space-4);
    background: var(--bg-secondary);
  }
  .plan-header {
    display: flex;
    justify-content: space-between;
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    margin-bottom: var(--space-2);
  }
  .plan-progress {
    color: var(--text-secondary);
    font-weight: 400;
    font-family: var(--font-mono);
  }
  .plan-progress-bar {
    height: 2px;
    background: var(--border);
    border-radius: var(--radius-0);
    margin-bottom: var(--space-4);
    overflow: hidden;
  }
  .plan-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 200ms ease-out;
  }
  .plan-steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .plan-step {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    font-size: var(--text-base);
    color: var(--text);
  }
  .plan-step.failed {
    color: var(--text-error);
  }
  :global(.icon-success) { color: var(--text-success); flex-shrink: 0; margin-top: 2px; }
  :global(.icon-running) { color: var(--accent); flex-shrink: 0; margin-top: 2px; }
  :global(.icon-error) { color: var(--text-error); flex-shrink: 0; margin-top: 2px; }
  :global(.icon-pending) { color: var(--text-tertiary); flex-shrink: 0; margin-top: 2px; }
</style>
