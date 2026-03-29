<script lang="ts">
    /**
     * IconButton Component
     * Unified icon button with variants, intents, micro-interactions, and tooltip support.
     */
    import styles from './IconButton.module.css';
    import Tooltip from './Tooltip.svelte';
    import type {Snippet} from 'svelte';

    type TooltipPosition = 'top' | 'bottom' | 'left' | 'right';

  interface Props {
    /** Icon snippet (required) */
    icon: Snippet;
    /** Accessible label (required) */
    label: string;
    /** Tooltip text (defaults to label) */
    tooltip?: string;
    /** Tooltip position */
    tooltipPosition?: TooltipPosition;
    /** Visual style variant */
    variant?: 'ghost' | 'subtle' | 'solid' | 'transparent';
    /** Intent/danger level */
    intent?: 'default' | 'success' | 'warning' | 'danger';
    /** Size of the button */
    size?: 'xs' | 'sm' | 'md' | 'lg';
    /** Disabled state */
    disabled?: boolean;
    /** Loading state */
    loading?: boolean;
    /** Active/pressed state */
    active?: boolean;
    /** Whether to show tooltip */
    showTooltip?: boolean;
    /** Click handler */
    onclick?: (e: MouseEvent) => void;
    /** Button type */
    type?: 'button' | 'submit' | 'reset';
    /** Additional CSS class */
    class?: string;
  }

  let {
    icon,
    label,
    tooltip,
    tooltipPosition = 'top',
    variant = 'ghost',
    intent = 'default',
    size = 'md',
    disabled = false,
    loading = false,
    active = false,
    showTooltip = true,
    onclick,
    type = 'button',
    class: className = ''
  }: Props = $props();

  const tooltipText = $derived(tooltip ?? label);

  let isPressed = $state(false);

  function handleMouseDown() {
    isPressed = true;
  }

  function handleMouseUp() {
    isPressed = false;
  }

  function handleClick(e: MouseEvent) {
    if (!disabled && !loading && onclick) {
      onclick(e);
    }
  }

  let classes = $derived(
    [
      styles.btn,
      styles[variant],
      styles[intent],
      styles[size],
      disabled && styles.disabled,
      loading && styles.loading,
      active && styles.active,
      isPressed && styles.pressed,
      className
    ]
      .filter(Boolean)
      .join(' ')
  );

  // Check for reduced motion preference
  const prefersReducedMotion =
    typeof window !== 'undefined' &&
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    if (prefersReducedMotion) {
      // Add class to document root for reduced motion
      document.documentElement.classList.add('reduce-motion');
    }
  });
</script>

{#if showTooltip && !disabled}
  <Tooltip content={tooltipText} position={tooltipPosition}>
    <button
      class={classes}
      {disabled}
      {type}
      onclick={handleClick}
      onmousedown={handleMouseDown}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
      aria-label={label}
      aria-disabled={disabled || loading}
      aria-pressed={active}
      aria-busy={loading}
    >
      <span class={styles.iconWrapper}>
        {#if loading}
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
        {:else}
          {@render icon()}
        {/if}
      </span>
    </button>
  </Tooltip>
{:else}
  <button
    class={classes}
    {disabled}
    {type}
    onclick={handleClick}
    onmousedown={handleMouseDown}
    onmouseup={handleMouseUp}
    onmouseleave={handleMouseUp}
    aria-label={label}
    aria-disabled={disabled || loading}
    aria-pressed={active}
    aria-busy={loading}
  >
    <span class={styles.iconWrapper}>
      {#if loading}
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
      {:else}
        {@render icon()}
      {/if}
    </span>
  </button>
{/if}
