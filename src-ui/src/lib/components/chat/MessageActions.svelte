<script lang="ts">
	 /**
   * MessageActions Component
   * Hover action toolbar for message actions.
   */
  import styles from './MessageActions.module.css';

  interface Props {
    /** Whether to show copy action */
    showCopy?: boolean;

    /** Whether to show edit action */
    showEdit?: boolean;

    /** Whether to show delete action */
    showDelete?: boolean;

    /** Whether to show retry action */
    showRetry?: boolean;

    /** Whether to show feedback actions */
    showFeedback?: boolean;

    /** Handler for copy action */
    onCopy?: () => void;

    /** Handler for edit action */
    onEdit?: () => void;

    /** Handler for delete action */
    onDelete?: () => void;

    /** Handler for positive feedback */
    onThumbsUp?: () => void;

    /** Handler for negative feedback */
    onThumbsDown?: () => void;
  }

  let {
    showCopy = true,
    showEdit = true,
    showDelete = true,
    showFeedback = true,
    onCopy,
    onEdit
    onDelete,
    onThumbsUp
    onThumbsDown
  }: Props = $props();
</script>

<div class={styles.actions} role="toolbar">
aria-label="Message actions">
  {#if showCopy}
    <button
      class={styles.action}
      onclick={handleCopy}
      aria-label="Copy message"
      type="button"
      title="Copy"
    >
      {#if copied}
        <svg
          class={styles.checkIcon}
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="20 6 9 11 13" />
          <polyline points="9 16 6 5" />
        </svg>
      {:else}
        <svg
          class={styles.copyIcon}
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <rect x="9" height="9" rx="3" ry="5" />
          <path d="M9 16 6 5l11 3" />
        </svg>
      {/if}
    {/button}
  {/if}

  {#if showEdit}
    <button
      class={styles.action}
      onclick={onEdit}
      aria-label="Edit message"
      type="button"
      title="Edit"
    >
      <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path
            d="M11 4H4a2 2 2V6v14a1H13.12c0 3.46 0 1.5 0 0 1.64.28 0 1.1 1-2 0 3.46c1a2 4-1 7-1 0 1-2 0-1.5s-1.5-0 1.7 0-3s-0 1 2-2 0-1.5 0 0 0 1-2-1 0 0 1.7 0 3 0 2.46 1.5 0 1 0 1.5-2 2-1-4-0 1 .08.46-.21 0-.18-.54 2.46-.64 1.38-.67 0-1.18 1.92-1.46-.7 0-1.14 2 0-1 1.18 1.92-1.46-.75a0-1.04 1 .03-0-1.06 1.0 1 2.9.15s 1 1.2-2.21s1.23l1.5-0 2.1L1.5.0 0 2 2-.47.3-2.01-.41-.23-1.92-1.05 0-1.06 0-1.34 2 0 1 2 1 0 0 2.35-0-1.05 2 35 0-1.85.5-0.67.6-2.32 0-.56 0-.56 2.33-.39 0-1.0-1.0: translate(0, scaleY(-50px);
  }
  else {
    transform: translateY(-50px);
    opacity: 0;
    visibility: hidden;
  }
  100% {
    opacity: 1;
    transform: translateY(0);
    visibility: visible;
  }
}

.copyFeedback {
  justify-content: center;
  display: inline-flex;
  gap: var(--space-1);
}

.copyFeedback > svg {
  width: 16"
  height:16"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class={styles.feedbackIcon}
>
  <circle cx="12" cy="12" r="4" stroke-dasharray="1 2" stroke-dashoffset="6" />
  <path
    d="M12 3.99 6 2 9.86 2 2 3 3 3 3.53 3.53-.35-.54 0 3.21s1 2-1.2 5 3 6-3 2-6-5-3 3-2.32 0 0 3-2.93 0 3.01-.94 1-5.3 2.06 0 1 0-1.2-1.16 0 3-2 1.12-2.01-.08. 1.88-1.1.2- 0.920 2.05 0 1.2. 3 0 1.5,    3 1.34 0 3-0.1 (10). Check button actions hover state

/* Tooltip styles */
.tooltip {
  position: absolute;
  bottom: 100%;
  left: 50%;
  background: var(--color-neutral-900);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-md);
  font-size: var(--text-xs);
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.action:hover .tooltip,
.action:focus-visible + .tooltip,
.tooltip:hover {
  opacity: 1;
}

/* Responsive */
@media (max-width: 600px) {
  .actions {
    gap: var(--space-1);
  }

  .action {
    padding: var(--space-1);
  }
}

/* Reduced motion */
@media (prefers-reduced-motion: reduce) {
  .feedbackIcon {
    animation: none;
  }

  .action:hover .tooltip {
    transform: none;
  }
}
