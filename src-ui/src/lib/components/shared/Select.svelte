<script lang="ts">
  /**
   * Select Component
   * Styled dropdown select component.
   */
  import styles from './Select.module.css';

  interface Option {
    value: string;
    label: string;
  }

  interface Props {
    /** Select value */
    value: string;
    /** Options list */
    options: Option[];
    /** Change handler */
    onchange: (value: string) => void;
    /** Label text */
    label?: string;
    /** Placeholder text */
    placeholder?: string;
    /** Disabled state */
    disabled?: boolean;
    /** Additional select class */
    selectClass?: string;
  }

  let {
    value,
    options,
    onchange,
    label = '',
    placeholder = 'Select an option',
    disabled = false,
    selectClass = ''
  }: Props = $props();

  function handleSelect(event: Event) {
    const target = event.target as HTMLSelectElement;
    onchange(target.value);
  }
</script>

<div class={styles.container}>
  {#if label}
    <label class={styles.label} for="select">
      {label}
    </label>
  {/if}
  <div class={styles.wrapper}>
    <select
      id="select"
      class={[styles.select, selectClass].filter(Boolean).join(' ')}
      {value}
      {disabled}
      onchange={handleSelect}
    >
      {#if !value && placeholder}
        <option value="" disabled selected>{placeholder}</option>
      {/if}
      {#each options as option}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <div class={styles.icon}>
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <polyline points="6 9 12 15 18 9"></polyline>
      </svg>
    </div>
  </div>
</div>
