<script lang="ts">
    /**
     * Button Component
     * Primary, secondary, ghost, and danger variants with enhanced micro-interactions.
     */
    import styles from './Button.module.css';
    import Tooltip from './Tooltip.svelte';
    import type {Snippet} from 'svelte';

    interface Props {
    /** Button variant - use 'accent' for main CTAs only */
    variant?: 'primary' | 'accent' | 'secondary' | 'ghost' | 'danger';
    /** Button size */
    size?: 'default' | 'sm' | 'lg';
    /** Disabled state */
    disabled?: boolean;
    /** Loading state */
    loading?: boolean;
    /** Icon only */
    iconOnly?: boolean;
    /** Click handler */
    onclick?: () => void;
    /** Type attribute */
    type?: 'button' | 'submit' | 'reset';
    /** Icon snippet */
    icon?: Snippet;
    /** Children */
    children?: Snippet;
    /** Tooltip text (shows on hover) */
    tooltip?: string;
    /** Animation style */
    animation?: 'none' | 'subtle' | 'expressive';
    /** Full width button */
    fullWidth?: boolean;
    /** Additional CSS class */
    class?: string;
    /** ARIA label for accessibility */
    'aria-label'?: string;
    /** ARIA describedby */
    'aria-describedby'?: string;
  }

  let {
    variant = 'primary',
    size = 'default',
    disabled = false,
    loading = false,
    iconOnly = false,
    onclick,
    type = 'button',
    icon,
    children,
    tooltip,
    animation = 'subtle',
    fullWidth = false,
    class: className = '',
    'aria-label': ariaLabel,
    'aria-describedby': ariaDescribedby
  }: Props = $props();

  let isPressed = $state(false);

  function handleMouseDown() {
    isPressed = true;
  }

  function handleMouseUp() {
    isPressed = false;
  }

  function handleClick() {
    if (!disabled && !loading && onclick) {
      onclick();
    }
  }

  let classes = $derived(
    [
      styles.btn,
      styles[variant],
      styles[size],
      styles[animation],
      disabled && styles.disabled,
      loading && styles.loading,
      iconOnly && styles.iconOnly,
      fullWidth && styles.fullWidth,
      isPressed && styles.pressed,
      className
    ]
      .filter(Boolean)
      .join(' ')
  );

  const showTooltip = $derived(tooltip && !disabled);
</script>

{#snippet renderSpinner()}
  <svg
    class={styles.spinner}
    xmlns="http://www.w3.org/2000/svg"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
  >
    <circle cx="12" cy="12" r="10" stroke-opacity="0.25"></circle>
    <path d="M12 2a10 10 0 0 1 10 10" stroke-opacity="0.75">
      <animateTransform
        attributeName="transform"
        type="rotate"
        from="0 12 12"
        to="360 12 12"
        dur="1s"
        repeatCount="indefinite"
      />
    </path>
  </svg>
{/snippet}

{#snippet renderButton()}
  <button
    class={classes}
    {disabled}
    {type}
    onclick={handleClick}
    onmousedown={handleMouseDown}
    onmouseup={handleMouseUp}
    onmouseleave={handleMouseUp}
    aria-disabled={disabled || loading}
    aria-busy={loading}
    aria-label={ariaLabel}
    aria-describedby={ariaDescribedby}
  >
    {#if loading}
      {@render renderSpinner()}
    {:else if icon}
      <span class={styles.btnContent}>
        {@render icon()}
      </span>
    {/if}
    {#if !iconOnly && children}
      <span class={styles.btnContent}>
        {@render children()}
      </span>
    {/if}
  </button>
{/snippet}

{#if showTooltip && tooltip}
  <Tooltip content={tooltip} position="top">
    {@render renderButton()}
  </Tooltip>
{:else}
  {@render renderButton()}
{/if}
