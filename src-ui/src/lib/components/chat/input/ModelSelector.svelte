<script lang="ts">
    /**
     * ModelSelector Component
     * Dropdown menu for selecting AI models for code generation.
     */
    import styles from './ModelSelector.module.css';
    import {availableModelsByOperation} from '$lib/stores/configStore';
    import {CheckIcon, ChevronDownIcon, SettingsIcon} from '$lib/icons';
    import type {Model} from '$lib/types/config';

    interface Props {
    selectedModelId: string | null;
    onModelChange: (modelId: string) => void;
    onOpenSettings?: () => void;
  }

  let { selectedModelId, onModelChange, onOpenSettings }: Props = $props();

  let isOpen = $state(false);
  let selectedIndex = $state(0);
  let listRef: HTMLUListElement | undefined = $state();
  let buttonRef: HTMLButtonElement | undefined = $state();

  const codeModels = $derived($availableModelsByOperation['code'] || []);
  const hasModels = $derived(codeModels.length > 0);

  let selectedModel = $derived(
    selectedModelId ? codeModels.find((m) => m.id === selectedModelId) : null
  );

  $effect(() => {
    if (codeModels.length > 0) {
      selectedIndex = 0;
    }
  });

  function getModelDisplayName(model: Model): string {
    const words = model.name.split(' ');
    const shortName = words.length > 2 ? words.slice(0, 2).join(' ') : model.name;
    return `${model.providerId}/${shortName}`;
  }

  function toggleMenu() {
    if (!hasModels && onOpenSettings) {
      onOpenSettings();
      return;
    }
    isOpen = !isOpen;
  }

  function closeMenu() {
    isOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isOpen) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, codeModels.length - 1);
      scrollToSelected();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollToSelected();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (codeModels[selectedIndex]) {
        selectModel(codeModels[selectedIndex].id);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeMenu();
    } else if (e.key === 'Tab') {
      closeMenu();
    }
  }

  function scrollToSelected() {
    const selectedEl = listRef?.querySelector('[data-selected="true"]');
    selectedEl?.scrollIntoView({ block: 'nearest' });
  }

  function selectModel(modelId: string) {
    onModelChange(modelId);
    closeMenu();
    buttonRef?.focus();
  }

  function handleItemClick(modelId: string, index: number) {
    selectedIndex = index;
    selectModel(modelId);
  }

  $effect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeydown);
      return () => document.removeEventListener('keydown', handleKeydown);
    }
  });

  function handleClickOutside(e: MouseEvent) {
    if (isOpen && buttonRef && !buttonRef.contains(e.target as Node)) {
      const menuEl = document.querySelector('[data-menu="model-selector"]');
      if (menuEl && !menuEl.contains(e.target as Node)) {
        closeMenu();
      }
    }
  }

  $effect(() => {
    if (isOpen) {
      document.addEventListener('click', handleClickOutside);
      return () => document.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<div class={styles.container}>
  <button
    bind:this={buttonRef}
    class={styles.trigger}
    onclick={toggleMenu}
    aria-label={hasModels ? 'Select AI model for code generation' : 'Configure models'}
    aria-expanded={isOpen}
    aria-haspopup="listbox"
    type="button"
  >
    {#if hasModels}
      {#if selectedModel}
        <span class={styles.modelText}>{getModelDisplayName(selectedModel)}</span>
      {:else}
        <span class={styles.modelText}>Select model</span>
      {/if}
      <ChevronDownIcon
        class={[styles.chevron, isOpen && styles.chevronOpen].filter(Boolean).join(' ')}
        size={12}
      />
    {:else}
      <SettingsIcon size={12} />
      <span class={styles.modelText}>Configure models</span>
    {/if}
  </button>

  {#if isOpen && hasModels}
    <div
      class={styles.menu}
      data-menu="model-selector"
      role="listbox"
      aria-label="Available models"
    >
      <ul bind:this={listRef} class={styles.list}>
        {#each codeModels as model, index}
          {@const isSelected = model.id === selectedModelId}
          {@const isFocused = index === selectedIndex}
          <button
            class={[styles.item, isSelected && styles.selected, isFocused && styles.focused]
              .filter(Boolean)
              .join(' ')}
            role="option"
            aria-selected={isSelected}
            data-selected={isSelected}
            data-focused={isFocused}
            onclick={() => handleItemClick(model.id, index)}
            onmouseenter={() => (selectedIndex = index)}
            type="button"
          >
              <span class={styles.checkmark}>
                {#if isSelected}
                  <CheckIcon size={10} strokeWidth={3} />
                {/if}
              </span>
              <span class={styles.itemName}>{model.name}</span>
              <span class={styles.itemMeta}>
                <span class={styles.provider}>{model.providerId}</span>
              </span>
          </button>
        {/each}
      </ul>

      <div class={styles.footer}>
        <span class={styles.hint}>
          <kbd>↑↓</kbd> navigate
          <kbd>Enter</kbd> select
          <kbd>Esc</kbd> close
        </span>
      </div>
    </div>
  {/if}
</div>
