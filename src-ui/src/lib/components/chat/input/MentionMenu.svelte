<script lang="ts">
    /**
     * MentionMenu Component
     * Menu for selecting files, agents, or context to mention, triggered by "@".
     */
    import styles from './MentionMenu.module.css';

    interface MentionItem {
    id: string;
    name: string;
    type: 'file' | 'agent' | 'context';
    icon?: string;
    description?: string;
  }

  interface Props {
    /** Current search query */
    query: string;
    /** Available items to display */
    items: MentionItem[];
    /** Handler when an item is selected */
    onSelect: (item: MentionItem) => void;
    /** Handler to close the menu */
    onClose: () => void;
  }

  let { query, items, onSelect, onClose }: Props = $props();

  let selectedIndex = $state(0);
  let listRef: HTMLUListElement | undefined = $state();

  // Reset selection when items change
  $effect(() => {
    if (items.length > 0) {
      selectedIndex = 0;
    }
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      scrollToSelected();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollToSelected();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (items[selectedIndex]) {
        onSelect(items[selectedIndex]);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function scrollToSelected() {
    const selectedEl = listRef?.querySelector('[data-selected="true"]');
    selectedEl?.scrollIntoView({ block: 'nearest' });
  }

  function handleItemClick(item: MentionItem, index: number) {
    selectedIndex = index;
    onSelect(item);
  }

  // Attach keyboard listener
  $effect(() => {
    document.addEventListener('keydown', handleKeydown);
    return () => document.removeEventListener('keydown', handleKeydown);
  });

  // Get type label
  function getTypeLabel(type: string): string {
    const labels: Record<string, string> = {
      file: 'File',
      agent: 'Agent',
      context: 'Context'
    };
    return labels[type] || type;
  }

  // Get type icon
  function getTypeIcon(type: string): string {
    return type;
  }
</script>

{#if items.length > 0}
  <div
    class={styles.menu}
    role="listbox"
    aria-label="Available mentions"
    aria-activedescendant={items.length > 0 ? `mention-option-${selectedIndex}` : undefined}
    tabindex="-1"
  >
    <div class={styles.header}>
      <span class={styles.title}>Mention</span>
      {#if query}
        <span class={styles.query}>@{query}</span>
      {/if}
    </div>

    <ul bind:this={listRef} class={styles.list}>
      {#each items as item, index}
        <button
          id="mention-option-{index}"
          class={[styles.item, index === selectedIndex && styles.selected]
            .filter(Boolean)
            .join(' ')}
          role="option"
          aria-selected={index === selectedIndex}
          data-selected={index === selectedIndex}
          onclick={() => handleItemClick(item, index)}
          onmouseenter={() => (selectedIndex = index)}
          type="button"
        >
          <span class={styles.itemIcon}>
            {#if item.type === 'file'}
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
                <polyline points="13 2 13 9 20 9" />
              </svg>
            {:else if item.type === 'agent'}
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="3" />
                <path
                  d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
                />
              </svg>
            {:else}
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <line x1="12" y1="16" x2="12" y2="12" />
                <line x1="12" y1="8" x2="12.01" y2="8" />
              </svg>
            {/if}
          </span>
          <span class={styles.itemContent}>
            <span class={styles.itemName}>{item.name}</span>
            {#if item.description}
              <span class={styles.itemDesc}>{item.description}</span>
            {/if}
          </span>
          <span class={styles.itemType}>{getTypeLabel(item.type)}</span>
        </button>
      {/each}
    </ul>

    <div class={styles.footer}>
      <span class={styles.hint}>
        <kbd>↑↓</kbd> to navigate
        <kbd>Enter</kbd> to select
        <kbd>Esc</kbd> to close
      </span>
    </div>
  </div>
{:else}
  <div class={styles.empty}>
    <span class={styles.emptyText}>No items found for "{query}"</span>
  </div>
{/if}
