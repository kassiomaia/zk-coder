<script lang="ts">
    /**
     * Modal Component
     * Reusable modal with overlay wrapper, header, body, and footer slots.
     * Matches the prototype pattern: .modal-overlay wraps .modal
     */
    import styles from './Modal.module.css';
    import IconButton from './IconButton.svelte';
    import {CloseIcon} from '$lib/icons';
    import type {Snippet} from 'svelte';

    // Track number of open modals globally to properly manage body scroll
  // Use a regular variable with manual tracking since we need module-level state
  let openModalCount = 0;
  const bodyScrollOriginalStyle = new WeakMap<Document, string>();

  function updateBodyScroll(isOpen: boolean) {
    if (isOpen) {
      if (openModalCount === 0) {
        // First modal opening - store original style and hide scroll
        bodyScrollOriginalStyle.set(document, document.body.style.overflow);
        document.body.style.overflow = 'hidden';
      }
      openModalCount++;
    } else {
      openModalCount = Math.max(0, openModalCount - 1);
      if (openModalCount === 0) {
        // Last modal closed - restore original style
        const original = bodyScrollOriginalStyle.get(document);
        document.body.style.overflow = original ?? '';
        bodyScrollOriginalStyle.delete(document);
      }
    }
  }

  interface Props {
    /** Modal title */
    title?: string;
    /** Open state */
    open: boolean;
    /** Close handler */
    onclose: () => void;
    /** Size variant */
    size?: 'default' | 'lg' | 'xl' | 'full';
    /** Show close button */
    showClose?: boolean;
    /** Additional class for modal content */
    contentClass?: string;
    /** Stacked modal - higher z-index for modals opened from other modals */
    stacked?: boolean;
    /** ID of element that describes the modal content */
    describedby?: string;
    /** Children */
    children?: Snippet;
    /** Footer snippet */
    footer?: Snippet;
  }

  let {
    title = '',
    open,
    onclose,
    size = 'default',
    showClose = true,
    contentClass = '',
    stacked = false,
    describedby,
    children,
    footer
  }: Props = $props();

  let wrapperClasses = $derived(
    [styles.overlay, stacked && styles.top, open && styles.open].filter(Boolean).join(' ')
  );

  let modalClasses = $derived(
    [styles.modal, styles[size], open && styles.open].filter(Boolean).join(' ')
  );

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && open) {
      onclose();
    }
  }

  // Prevent body scroll when modal is open
  $effect(() => {
    updateBodyScroll(open);

    if (open) {
      document.addEventListener('keydown', handleKeydown);
    }

    return () => {
      document.removeEventListener('keydown', handleKeydown);
      updateBodyScroll(false);
    };
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class={wrapperClasses}
    onclick={(e) => {
      // Close only when clicking the overlay, not the modal content
      if (e.target === e.currentTarget) onclose();
    }}
    onkeydown={(e) => {
      // Allow closing with Escape key
      if (e.key === 'Escape') onclose();
    }}
    role="presentation"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class={modalClasses}
      role="dialog"
      aria-modal="true"
      aria-labelledby={title ? 'modal-title' : undefined}
      aria-describedby={describedby}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      {#if title || showClose}
        <header class={styles.header}>
          {#if title}
            <h2 id="modal-title" class={styles.title}>{title}</h2>
          {/if}
          {#if showClose}
            <IconButton
              label="Close modal"
              tooltip="Close"
              tooltipPosition="bottom"
              variant="ghost"
              size="sm"
              onclick={onclose}
              icon={closeIcon}
            />
          {/if}
        </header>
      {/if}
      <div class={[styles.body, contentClass === 'no-padding-body' ? styles['no-padding-body'] : contentClass].filter(Boolean).join(' ')}>
        {#if children}
          {@render children()}
        {/if}
      </div>
      {#if footer}
        <footer class={styles.footer}>
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}

{#snippet closeIcon()}
  <CloseIcon size={20} />
{/snippet}

