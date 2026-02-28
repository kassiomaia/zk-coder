# Svelte Styling Guidelines

## CSS Modules

### Basic Usage

Every component has a matching CSS module:

```svelte
<!-- Button.svelte -->
<script lang="ts">
  import styles from './Button.module.css';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
  }

  let { variant = 'primary', size = 'md', disabled = false }: Props = $props();

  let classes = $derived(
    [
      styles.button,
      styles[variant],
      styles[size],
      disabled && styles.disabled
    ].filter(Boolean).join(' ')
  );
</script>

<button class={classes} {disabled}>
  {@render children()}
</button>

<style>/* Styles in separate .module.css file */</style>
```

```css
/* Button.module.css */
.button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-xs);
  font-family: var(--font-family);
  font-weight: 500;
  border: none;
  cursor: pointer;
  transition: all 0.2s ease;
}

/* Variants */
.primary {
  background: var(--color-primary);
  color: var(--color-on-primary);
}

.primary:hover:not(.disabled) {
  background: var(--color-primary-hover);
}

.secondary {
  background: var(--color-surface);
  color: var(--color-text);
  border: 1px solid var(--color-border);
}

.ghost {
  background: transparent;
  color: var(--color-text);
}

.ghost:hover:not(.disabled) {
  background: var(--color-surface);
}

/* Sizes */
.sm {
  padding: var(--spacing-xs) var(--spacing-sm);
  font-size: var(--font-size-sm);
  border-radius: var(--radius-sm);
}

.md {
  padding: var(--spacing-sm) var(--spacing-md);
  font-size: var(--font-size-md);
  border-radius: var(--radius-md);
}

.lg {
  padding: var(--spacing-md) var(--spacing-lg);
  font-size: var(--font-size-lg);
  border-radius: var(--radius-lg);
}

/* States */
.disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

## Design System Tokens

### Color Tokens

Always use CSS variables from the design system:

```css
/* design-system.css */
:root {
  /* Brand colors */
  --color-primary: #6366f1;
  --color-primary-hover: #4f46e5;
  --color-primary-active: #4338ca;

  /* Semantic colors */
  --color-success: #22c55e;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-info: #3b82f6;

  /* Background colors */
  --color-background: #0f172a;
  --color-surface: #1e293b;
  --color-surface-hover: #334155;

  /* Text colors */
  --color-text: #f8fafc;
  --color-text-secondary: #94a3b8;
  --color-text-muted: #64748b;

  /* Border colors */
  --color-border: #334155;
  --color-border-focus: #6366f1;
}

/* Light theme overrides */
[data-theme="light"] {
  --color-background: #f8fafc;
  --color-surface: #ffffff;
  --color-text: #0f172a;
  --color-text-secondary: #475569;
  --color-border: #e2e8f0;
}
```

### Spacing Scale

```css
:root {
  --spacing-0: 0;
  --spacing-1: 0.25rem;  /* 4px */
  --spacing-2: 0.5rem;   /* 8px */
  --spacing-3: 0.75rem;  /* 12px */
  --spacing-4: 1rem;     /* 16px */
  --spacing-5: 1.25rem;  /* 20px */
  --spacing-6: 1.5rem;   /* 24px */
  --spacing-8: 2rem;     /* 32px */
  --spacing-10: 2.5rem;  /* 40px */
  --spacing-12: 3rem;    /* 48px */
  --spacing-16: 4rem;    /* 64px */
}
```

### Typography Scale

```css
:root {
  /* Font families */
  --font-family: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;

  /* Font sizes */
  --font-size-xs: 0.75rem;   /* 12px */
  --font-size-sm: 0.875rem;  /* 14px */
  --font-size-base: 1rem;    /* 16px */
  --font-size-lg: 1.125rem;  /* 18px */
  --font-size-xl: 1.25rem;   /* 20px */
  --font-size-2xl: 1.5rem;   /* 24px */
  --font-size-3xl: 1.875rem; /* 30px */

  /* Line heights */
  --line-height-tight: 1.25;
  --line-height-normal: 1.5;
  --line-height-relaxed: 1.75;
}
```

### Border Radius

```css
:root {
  --radius-none: 0;
  --radius-sm: 0.25rem;  /* 4px */
  --radius-md: 0.5rem;   /* 8px */
  --radius-lg: 0.75rem;  /* 12px */
  --radius-xl: 1rem;     /* 16px */
  --radius-full: 9999px;
}
```

### Shadows

```css
:root {
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1);
  --shadow-xl: 0 20px 25px -5px rgb(0 0 0 / 0.1);
}
```

## Utility Patterns

### Class Composition Helper

```typescript
// $lib/utils/classes.ts
export function cn(...classes: (string | boolean | undefined | null)[]): string {
  return classes.filter(Boolean).join(' ');
}
```

```svelte
<script lang="ts">
  import { cn } from '$lib/utils/classes';
  import styles from './Card.module.css';

  interface Props {
    variant?: 'elevated' | 'outlined' | 'filled';
    padding?: 'none' | 'sm' | 'md' | 'lg';
    class?: string;
  }

  let { variant = 'elevated', padding = 'md', class: className }: Props = $props();
</script>

<div class={cn(styles.card, styles[variant], styles[`padding-${padding}`], className)}>
  {@render children()}
</div>
```

### CSS Variables for Dynamic Styles

```svelte
<script lang="ts">
  interface Props {
    color?: string;
    size?: number;
  }

  let { color = 'var(--color-primary)', size = 16 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  style:color={color}
>
  <!-- icon paths -->
</svg>
```

## Responsive Design

### Breakpoint Variables

```css
/* design-system.css */
:root {
  --breakpoint-sm: 640px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;
}
```

### Media Query Usage

```css
/* Sidebar.module.css */
.sidebar {
  width: 100%;
  height: auto;
}

@media (min-width: 768px) {
  .sidebar {
    width: 240px;
    height: 100vh;
  }
}

@media (min-width: 1024px) {
  .sidebar {
    width: 280px;
  }
}
```

### Container Queries (Modern)

```css
/* Card.module.css */
.card-container {
  container-type: inline-size;
}

.card {
  display: flex;
  flex-direction: column;
}

@container (min-width: 400px) {
  .card {
    flex-direction: row;
  }
}
```

## Animation Guidelines

### Transition Tokens

```css
:root {
  --transition-fast: 150ms ease;
  --transition-normal: 200ms ease;
  --transition-slow: 300ms ease;
}
```

### Micro-interactions

```css
/* Button.module.css */
.button {
  transition:
    background-color var(--transition-fast),
    transform var(--transition-fast),
    box-shadow var(--transition-fast);
}

.button:active:not(.disabled) {
  transform: scale(0.98);
}

.button:focus-visible {
  outline: 2px solid var(--color-border-focus);
  outline-offset: 2px;
}
```

### Enter/Exit Animations

```css
/* Modal.module.css */
.modal {
  animation: fadeIn var(--transition-normal);
}

.modal.closing {
  animation: fadeOut var(--transition-normal);
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fadeOut {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}
```

## Best Practices

1. **Always use CSS modules**: Never use global styles in components
2. **Reference design tokens**: Use CSS variables, avoid hardcoded values
3. **Semantic naming**: Use purpose-based class names (`primary`, `disabled`)
4. **Mobile-first**: Write base styles for mobile, enhance for larger screens
5. **Focus states**: Always include `:focus-visible` styles for accessibility
6. **Consistent transitions**: Use transition tokens for uniform animations
