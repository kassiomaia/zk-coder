<script lang="ts">
    /**
     * ChatScrollButton Component
     * Scroll-to-bottom button for chat area.
     */
    import styles from './ChatArea.module.css';
    import {isLlmStreaming} from '$lib/stores/chat';
    import {ArrowDownIcon} from '$lib/icons';

    interface Props {
    /** Whether to show the button */
    show: boolean;

    /** Click handler */
    onclick: () => void;
  }

  let { show, onclick }: Props = $props();
</script>

{#if show}
  <button
    class={[styles.scrollToBottom, $isLlmStreaming && styles.pulsing, styles.secondaryElements]
      .filter(Boolean)
      .join(' ')}
    onclick={onclick}
    aria-label="Scroll to latest messages"
    type="button"
  >
    <ArrowDownIcon size={20} />
  </button>
{/if}
