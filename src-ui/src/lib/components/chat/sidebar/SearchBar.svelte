<script lang="ts">
    /**
     * SearchBar Component
     * Search input for filtering conversations.
     */
    import styles from './SearchBar.module.css';
    import {CloseIcon, SearchIcon} from '$lib/icons';

    interface Props {
    /** Current search value */
    value?: string;
    /** Input handler */
    oninput?: (value: string) => void;
    /** Placeholder text */
    placeholder?: string;
    /** Disabled state */
    disabled?: boolean;
  }

  let { value = '', oninput, placeholder = 'Search...', disabled = false }: Props = $props();

  let inputRef: HTMLInputElement | undefined = $state();

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    oninput?.(target.value);
  }

  function clearSearch() {
    oninput?.('');
    inputRef?.focus();
  }
</script>

<div class={styles.search}>
  <SearchIcon class={styles.icon} size={16} aria-hidden={true} />

  <input
    bind:this={inputRef}
    class={styles.input}
    type="search"
    {value}
    {placeholder}
    {disabled}
    oninput={handleInput}
    aria-label="Search conversations"
  />

  {#if value}
    <button class={styles.clear} onclick={clearSearch} aria-label="Clear search" type="button">
      <CloseIcon size={14} />
    </button>
  {/if}
</div>
