<!--
  ComponentName.svelte
  Brief description of the component's purpose.

  @example
  <ComponentName variant="primary" onclick={handleClick}>
    Click me
  </ComponentName>
-->
<script lang="ts">
  // ============================================
  // 1. IMPORTS
  // ============================================
  import styles from './ComponentName.module.css';

  // ============================================
  // 2. TYPES
  // ============================================
  /** Visual style variant */
  type Variant = 'primary' | 'secondary' | 'ghost';
  /** Size of the component */
  type Size = 'sm' | 'md' | 'lg';

  interface Props {
    /** Visual style variant */
    variant?: Variant;
    /** Size of the component */
    size?: Size;
    /** Whether the component is disabled */
    disabled?: boolean;
    /** Loading state */
    loading?: boolean;
    /** Click handler */
    onclick?: () => void;
    /** Child content */
    children?: import('svelte').Snippet;
    /** Icon content */
    icon?: import('svelte').Snippet;
  }

  // ============================================
  // 3. PROPS
  // ============================================
  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    onclick,
    children,
    icon
  }: Props = $props();

  // ============================================
  // 4. STATE
  // ============================================
  let isHovered = $state(false);
  let isFocused = $state(false);

  // ============================================
  // 5. DERIVED VALUES
  // ============================================
  let classes = $derived(
    [
      styles.component,
      styles[variant],
      styles[size],
      disabled && styles.disabled,
      loading && styles.loading,
      isHovered && styles.hovered,
      isFocused && styles.focused
    ].filter(Boolean).join(' ')
  );

  // ============================================
  // 6. EFFECTS
  // ============================================
  $effect(() => {
    // Reset hover/focus when disabled
    if (disabled) {
      isHovered = false;
      isFocused = false;
    }
  });

  // ============================================
  // 7. EVENT HANDLERS
  // ============================================
  function handleClick(e: MouseEvent) {
    if (!disabled && !loading && onclick) {
      onclick();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleClick(e as unknown as MouseEvent);
    }
  }

  // ============================================
  // 8. HELPER FUNCTIONS
  // ============================================
  function formatValue(value: unknown): string {
    return String(value);
  }
</script>

<!-- ============================================
     9. TEMPLATE
     ============================================ -->
<button
  class={classes}
  type="button"
  {disabled}
  aria-disabled={disabled || loading}
  aria-busy={loading}
  onclick={handleClick}
  onkeydown={handleKeydown}
  onmouseenter={() => isHovered = true}
  onmouseleave={() => isHovered = false}
  onfocus={() => isFocused = true}
  onblur={() => isFocused = false}
>
  {#if loading}
    <span class={styles.spinner} aria-hidden="true" />
  {:else if icon}
    <span class={styles.icon}>
      {@render icon()}
    </span>
  {/if}

  {#if children}
    <span class={styles.label}>
      {@render children()}
    </span>
  {/if}
</button>
