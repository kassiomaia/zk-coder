# Button Component

Interactive button element with multiple variants and sizes.

## TypeScript Interface

```typescript
interface ButtonProps {
  /** Visual style variant */
  variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';

  /** Size of the button */
  size?: 'sm' | 'md' | 'lg';

  /** Disabled state */
  disabled?: boolean;

  /** Loading state */
  loading?: boolean;

  /** Full width */
  fullWidth?: boolean;

  /** Icon position */
  iconPosition?: 'left' | 'right';

  /** Click handler */
  onclick?: () => void;

  /** Button type */
  type?: 'button' | 'submit' | 'reset';

  /** Child content */
  children?: import('svelte').Snippet;

  /** Icon content */
  icon?: import('svelte').Snippet;
}
```

---

## Design Tokens

```css
/* Base */
font-family: var(--font-sans);
font-weight: var(--font-medium);
border-radius: var(--radius-md);
transition: all var(--transition-fast);

/* Sizes */
/* sm: padding: var(--space-1) var(--space-3); font-size: var(--text-xs); */
/* md: padding: var(--space-2) var(--space-4); font-size: var(--text-sm); */
/* lg: padding: var(--space-3) var(--space-6); font-size: var(--text-base); */
```

---

## Variants

| Variant | Background | Text | Border | Hover Background |
|---------|------------|------|--------|------------------|
| Primary | `--color-primary-500` | `--color-neutral-0` | none | `--color-primary-600` |
| Secondary | `--color-surface` | `--color-text-primary` | `--color-border-primary` | `--color-surface-hover` |
| Ghost | transparent | `--color-text-primary` | none | `--color-surface-hover` |
| Destructive | `--color-error` | `--color-neutral-0` | none | `#dc2626` |

---

## States

| State | Visual |
|-------|--------|
| Default | Normal appearance |
| Hover | Darker background |
| Active | Even darker background |
| Focus | Focus ring |
| Disabled | 50% opacity, no pointer events |
| Loading | Spinner icon, disabled |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './Button.module.css';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost' | 'destructive';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    loading?: boolean;
    fullWidth?: boolean;
    type?: 'button' | 'submit' | 'reset';
    onclick?: () => void;
    children?: import('svelte').Snippet;
    icon?: import('svelte').Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    disabled = false,
    loading = false,
    fullWidth = false,
    type = 'button',
    onclick,
    children,
    icon
  }: Props = $props();

  let classes = $derived(
    [
      styles.button,
      styles[variant],
      styles[size],
      fullWidth && styles.fullWidth,
      loading && styles.loading
    ].filter(Boolean).join(' ')
  );

  function handleClick(e: MouseEvent) {
    if (!disabled && !loading && onclick) {
      onclick();
    }
  }
</script>

<button
  class={classes}
  type={type}
  disabled={disabled || loading}
  aria-busy={loading}
  aria-disabled={disabled || loading}
  onclick={handleClick}
>
  {#if loading}
    <span class={styles.spinner} aria-hidden="true" />
  {:else if icon}
    {@render icon()}
  {/if}

  {#if children}
    <span class={styles.label}>{@render children()}</span>
  {/if}
</button>
```

```css
/* Button.module.css */
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  font-family: var(--font-sans);
  font-weight: var(--font-medium);
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
  text-decoration: none;
}

.button:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Variants */
.primary {
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
}

.primary:hover:not(:disabled) {
  background: var(--color-primary-600);
}

.primary:active:not(:disabled) {
  background: var(--color-primary-700);
}

.secondary {
  background: var(--color-surface);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
}

.secondary:hover:not(:disabled) {
  background: var(--color-surface-hover);
}

.secondary:active:not(:disabled) {
  background: var(--color-surface-active);
}

.ghost {
  background: transparent;
  color: var(--color-text-primary);
}

.ghost:hover:not(:disabled) {
  background: var(--color-surface-hover);
}

.destructive {
  background: var(--color-error);
  color: var(--color-neutral-0);
}

.destructive:hover:not(:disabled) {
  background: #dc2626;
}

/* Sizes */
.sm {
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  border-radius: var(--radius-sm);
}

.md {
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-sm);
}

.lg {
  padding: var(--space-3) var(--space-6);
  font-size: var(--text-base);
  border-radius: var(--radius-lg);
}

/* Modifiers */
.fullWidth {
  width: 100%;
}

.loading {
  cursor: wait;
}

/* Elements */
.label {
  /* Ensure label doesn't shrink */
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Disabled state */
.button:disabled {
  opacity: var(--opacity-disabled);
  cursor: not-allowed;
}
```

---

## Accessibility

- Use semantic `<button>` element
- Include `aria-busy` when loading
- Include `aria-disabled` when disabled
- Provide `type` attribute (default: "button")
- Ensure focus-visible styles
- Minimum 44px touch target for sm/md sizes
