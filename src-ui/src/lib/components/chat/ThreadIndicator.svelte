<script lang="ts">
    /**
     * ThreadIndicator Component
     * Shows a badge indicating which thread a message belongs to.
     * Clickable to navigate to the thread.
     */
    import styles from './ThreadIndicator.module.css';
    import type {Thread} from '$lib/types/thread';

    interface Props {
    /** The thread the message belongs to */
    thread: Thread;
    /** Handler to navigate to the thread */
    onclick?: () => void;
    /** Compact mode for inline display */
    compact?: boolean;
  }

  let { thread, onclick, compact = false }: Props = $props();

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    onclick?.();
  }
</script>

<button
  class={[styles.indicator, compact && styles.compact].filter(Boolean).join(' ')}
  onclick={handleClick}
  aria-label="View thread: {thread.title}"
  type="button"
>
  <svg
    class={styles.icon}
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
  </svg>
  {#if !compact}
    <span class={styles.title}>{thread.title}</span>
  {/if}
</button>
