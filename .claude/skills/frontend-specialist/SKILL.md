---
name: frontend-specialist
description: Expert frontend developer for Coder AI Code Agent. Implements components following the design system, ensures accessibility compliance, and creates consistent, high-quality UI code. Use when creating or modifying any UI components, pages, layouts, or styling work.
---

# Frontend Specialist Skill

Expert guide for implementing the Coder AI Code Agent interface following the design system.

## Core Principles

1. **Design Tokens First**: Always use CSS variables from `design-system.css`, never hardcoded values
2. **Theme-Aware**: Use semantic tokens (`--color-text-primary`) not palette tokens (`--color-neutral-50`)
3. **Accessibility Built-In**: WCAG AA compliance required for all components
4. **Type-Safe**: All components with TypeScript `Props` interfaces
5. **Svelte 5 Runes**: Use `$state`, `$derived`, `$effect` patterns

## Design System Location

```
src-ui/src/lib/styles/
├── design-system.css    # All design tokens
├── global.css           # Global styles and resets
└── fonts.css            # Font face definitions

src-ui/src/lib/components/   # Component library
```

## Quick Reference

### Colors
```css
/* Backgrounds */
background: var(--color-bg-primary);      /* Main background */
background: var(--color-bg-secondary);    /* Sidebar, panels */
background: var(--color-surface);         /* Component surfaces */

/* Text */
color: var(--color-text-primary);         /* Primary text */
color: var(--color-text-secondary);       /* Secondary text */
color: var(--color-text-muted);           /* Disabled, hints */

/* Borders */
border-color: var(--color-border-primary);
```

### Spacing
```css
padding: var(--space-1);    /* 4px - tight */
padding: var(--space-2);    /* 8px - small */
padding: var(--space-3);    /* 12px - medium */
padding: var(--space-4);    /* 16px - standard */
padding: var(--space-6);    /* 24px - large */
gap: var(--space-3);        /* 12px - element gaps */
margin: var(--space-8);     /* 32px - sections */
```

### Typography
```css
font-size: var(--text-xs);            /* 12px */
font-size: var(--text-sm);            /* 14px */
font-size: var(--text-base);          /* 16px */
font-weight: var(--font-medium);      /* 500 */
font-weight: var(--font-semibold);    /* 600 */
line-height: var(--leading-normal);   /* 1.5 */
```

### Border Radius
```css
border-radius: var(--radius-sm);      /* 2px */
border-radius: var(--radius-md);      /* 6px */
border-radius: var(--radius-lg);      /* 8px */
border-radius: var(--radius-xl);      /* 12px */
border-radius: var(--radius-full);    /* 9999px (circular) */
```

## Sections

- [**Styleguide**](./styleguide/): Design tokens, colors, typography, theming, accessibility
- [**Layout**](./layout/): Application structure, sidebar modes, responsive design
- [**Components**](./components/): Component specifications with TypeScript interfaces
- [**State**](./state/): State management patterns and store architecture
- [**References**](./references/): HTML reference files for learning patterns
- [**Examples**](./examples/): Component and page templates

## Component Creation Checklist

Before creating any component:

- [ ] Check `design-system.css` for available tokens
- [ ] Review similar existing components for patterns
- [ ] Define TypeScript `Props` interface with JSDoc comments
- [ ] Create CSS module using only design tokens
- [ ] Include accessibility attributes (`role`, `aria-*`, keyboard navigation)
- [ ] Include `:focus-visible` styles
- [ ] Test in both themes (dark/light)
- [ ] Test responsive breakpoints
- [ ] Keep component under 200 lines (refactor if needed)

## Common Patterns

### Component Structure
```svelte
<script lang="ts">
  // 1. Imports
  import styles from './ComponentName.module.css';

  // 2. Types
  interface Props {
    variant?: 'primary' | 'secondary';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onclick?: () => void;
    children?: import('svelte').Snippet;
  }

  // 3. Props
  let { variant = 'primary', size = 'md', disabled = false, onclick, children }: Props = $props();

  // 4. State
  let isHovered = $state(false);

  // 5. Derived
  let classes = $derived(
    [styles.component, styles[variant], styles[size], disabled && styles.disabled]
      .filter(Boolean).join(' ')
  );

  // 6. Handlers
  function handleClick() {
    if (!disabled) onclick?.();
  }
</script>

<button
  class={classes}
  {disabled}
  onclick={handleClick}
  type="button"
  aria-disabled={disabled}
>
  {@render children?.()}
</button>
```

### CSS Module Pattern
```css
.component {
  /* Use tokens, never hardcoded values */
  padding: var(--space-3) var(--space-4);
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  background: var(--color-surface);
  border-radius: var(--radius-md);
  transition: all var(--transition-fast);
}

.component:hover:not(.disabled) {
  background: var(--color-surface-hover);
}

.component:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

.disabled {
  opacity: var(--opacity-disabled);
  cursor: not-allowed;
}
```

## Related Skills

- [svelte](../svelte/SKILL.md): Svelte 5 runes and patterns
- [tauri-svelte](../tauri-svelte/SKILL.md): Tauri + Svelte integration
