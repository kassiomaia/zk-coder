<script lang="ts">
    /**
     * ToolCallsView Component
     * Displays structured tool calls from agent execution.
     * Auto-scrolls to bottom on new calls.
     */
    import type {ToolCallRecord} from '$lib/types/agent';
    import ToolCallItem from './ToolCallItem.svelte';

    interface Props {
    toolCalls: ToolCallRecord[];
  }

  let { toolCalls }: Props = $props();
  let container: HTMLDivElement | undefined = $state();

  /** Auto-scroll to bottom when new tool calls arrive */
  $effect(() => {
    if (toolCalls.length > 0 && container) {
      container.scrollTop = container.scrollHeight;
    }
  });
</script>

<div class="tool-calls-view" bind:this={container} role="log" aria-label="Tool calls history">
  <div class="tool-calls-list">
    {#each toolCalls as record (record.id)}
      <ToolCallItem {record} />
    {:else}
      <div class="empty-state">
        <div class="empty-icon-wrapper">
          <span class="empty-char">&gt;_</span>
        </div>
        <span class="empty-title">Console clear</span>
        <span class="empty-subtitle">Tools called will appear here in sequence</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .tool-calls-view {
    flex: 1;
    min-height: 120px;
    overflow-y: auto;
    background: var(--color-bg-secondary);
    margin: 0;
    border-top: 1px solid var(--color-border-primary);
    scroll-behavior: smooth;
  }

  .tool-calls-list {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    padding: var(--space-2) 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: var(--space-2);
    color: var(--color-text-muted);
    padding: var(--space-12) 0;
    opacity: 0.6;
  }

  .empty-icon-wrapper {
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    font-weight: var(--font-semibold);
    color: var(--color-text-tertiary);
    margin-bottom: var(--space-2);
  }

  .empty-char {
    display: inline-block;
    animation: cursor-blink 1s step-end infinite;
    border-right: 2px solid currentColor;
    padding-right: 2px;
  }

  @keyframes cursor-blink {
    0%, 100% { border-color: transparent; }
    50% { border-color: var(--color-primary-400); }
  }

  .empty-title {
    font-size: var(--text-xs);
    font-weight: var(--font-medium);
    color: var(--color-text-secondary);
    letter-spacing: 0.02em;
  }

  .empty-subtitle {
    font-size: var(--text-2xs);
    opacity: 0.5;
    max-width: var(--space-40);
    text-align: center;
    line-height: var(--leading-snug);
  }

  /* Scrollbar styling */
  .tool-calls-view::-webkit-scrollbar {
    width: var(--space-1-5);
  }

  .tool-calls-view::-webkit-scrollbar-track {
    background: transparent;
  }

  .tool-calls-view::-webkit-scrollbar-thumb {
    background: var(--color-neutral-700);
    border-radius: var(--radius-full);
  }
</style>
