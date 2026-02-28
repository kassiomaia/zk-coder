# Theming Guidelines

Guidelines for implementing dark and light themes in the Coder application.

## Theme Implementation

Themes are controlled via the `data-theme` attribute on the root element:

```html
<html data-theme="dark">
  <!-- Dark mode content -->
</html>

<html data-theme="light">
  <!-- Light mode content -->
</html>
```

### Theme Toggle Implementation

```typescript
// $lib/stores/theme.ts
import { persisted } from '$lib/stores/persisted';

export type Theme = 'light' | 'dark' | 'system';

export const themePreference = persisted<Theme>('theme', 'system');

export function getSystemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function applyTheme(theme: Theme) {
  const effectiveTheme = theme === 'system' ? getSystemTheme() : theme;
  document.documentElement.setAttribute('data-theme', effectiveTheme);
}
```

```svelte
<!-- ThemeToggle.svelte -->
<script lang="ts">
  import { themePreference, applyTheme } from '$lib/stores/theme';

  let currentTheme = $state($themePreference);

  function toggleTheme() {
    const next = currentTheme === 'dark' ? 'light' : 'dark';
    currentTheme = next;
    themePreference.set(next);
    applyTheme(next);
  }
</script>

<button
  onclick={toggleTheme}
  aria-label={`Switch to ${currentTheme === 'dark' ? 'light' : 'dark'} theme`}
>
  {#if currentTheme === 'dark'}
    <SunIcon />
  {:else}
    <MoonIcon />
  {/if}
</button>
```

---

## Theme-Aware Tokens

The design system provides theme-aware tokens that automatically change based on the active theme.

### Available Tokens

| Token | Dark Mode | Light Mode |
|-------|-----------|------------|
| `--color-bg-primary` | #111827 | #ffffff |
| `--color-bg-secondary` | #1f2937 | #f9fafb |
| `--color-bg-tertiary` | #374151 | #f3f4f6 |
| `--color-bg-elevated` | #1f2937 | #ffffff |
| `--color-text-primary` | #e2e8f0 | #111827 |
| `--color-text-secondary` | #94a3b8 | #4b5563 |
| `--color-text-tertiary` | #64748b | #6b7280 |
| `--color-text-muted` | #475569 | #9ca3af |
| `--color-border-primary` | #374151 | #e5e7eb |
| `--color-border-secondary` | #4b5563 | #d1d5db |
| `--color-surface` | #1f2937 | #ffffff |
| `--color-surface-hover` | #374151 | #f3f4f6 |
| `--color-surface-active` | #4b5563 | #e5e7eb |

---

## Usage Guidelines

### Always Use Theme-Aware Tokens

```css
/* CORRECT - Works in both themes */
.component {
  background: var(--color-surface);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
}
```

### Never Hardcode Theme-Specific Colors

```css
/* INCORRECT - Breaks in one theme */
.component {
  background: #1f2937;    /* Too dark in light mode */
  color: #e2e8f0;         /* Too light in light mode */
}
```

### Fixed Elements (Exception)

Some elements maintain the same color in both themes:

```css
/* Code blocks are always dark */
.code-block {
  background: var(--color-code-bg);    /* #1e1e1e in both themes */
  color: var(--color-code-text);       /* #d4d4d4 in both themes */
}

/* Primary accent stays consistent */
.button-primary {
  background: var(--color-primary-500); /* #6366f1 in both themes */
}
```

---

## Testing Both Themes

### Manual Testing Checklist

- [ ] All text is readable (contrast check)
- [ ] Borders are visible
- [ ] Hover states work correctly
- [ ] Focus indicators are visible
- [ ] Selection highlighting works
- [ ] Shadows are appropriate
- [ ] Icons are visible
- [ ] Images/diagrams are clear

### Automated Testing

```typescript
// Example: Contrast ratio test
import { contrast } from 'chroma-js';

const textColor = getComputedStyle(element).getPropertyValue('color');
const bgColor = getComputedStyle(element).getPropertyValue('background-color');

const ratio = contrast(textColor, bgColor);
console.assert(ratio >= 4.5, `Contrast ratio ${ratio} is below WCAG AA`);
```

---

## Theme Transition

### Smooth Transitions

```css
/* Apply to root for smooth theme changes */
:root {
  --theme-transition: background-color 0.3s ease,
                      color 0.3s ease,
                      border-color 0.3s ease;
}

/* Apply to components */
.card {
  transition: var(--theme-transition);
}

/* Respect reduced motion preference */
@media (prefers-reduced-motion: reduce) {
  .card {
    transition: none;
  }
}
```

### Transition on Theme Change Only

```css
/* Only transition when theme attribute changes */
[data-theme] * {
  transition: none;
}

[data-theme].theme-transitioning * {
  transition: background-color 0.3s ease,
              color 0.3s ease,
              border-color 0.3s ease;
}
```

```typescript
// Add transitioning class during theme change
function applyTheme(theme: Theme) {
  const root = document.documentElement;
  root.classList.add('theme-transitioning');

  const effectiveTheme = theme === 'system' ? getSystemTheme() : theme;
  root.setAttribute('data-theme', effectiveTheme);

  setTimeout(() => {
    root.classList.remove('theme-transitioning');
  }, 300);
}
```

---

## Common Patterns

### Card Component

```css
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  color: var(--color-text-primary);
}

.card-header {
  border-bottom: 1px solid var(--color-border-primary);
}

.card-title {
  color: var(--color-text-primary);
}

.card-description {
  color: var(--color-text-secondary);
}
```

### Input Component

```css
.input {
  background: var(--color-bg-tertiary);
  border: 1px solid var(--color-border-primary);
  color: var(--color-text-primary);
}

.input:focus {
  border-color: var(--color-primary-500);
}

.input::placeholder {
  color: var(--color-text-muted);
}

.input:disabled {
  background: var(--color-bg-secondary);
  color: var(--color-text-muted);
}
```

### Button Component

```css
.button {
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  color: var(--color-text-primary);
}

.button:hover {
  background: var(--color-surface-hover);
}

.button:active {
  background: var(--color-surface-active);
}

.button-primary {
  background: var(--color-primary-500);
  border-color: var(--color-primary-500);
  color: var(--color-neutral-0);
}
```

---

## System Theme Detection

### Listen for System Theme Changes

```typescript
// $lib/stores/theme.ts
export function watchSystemTheme(callback: (theme: 'light' | 'dark') => void) {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

  const handler = (e: MediaQueryListEvent) => {
    callback(e.matches ? 'dark' : 'light');
  };

  mediaQuery.addEventListener('change', handler);

  return () => {
    mediaQuery.removeEventListener('change', handler);
  };
}
```

### CSS Media Query

```css
/* Apply styles based on system preference (before JS loads) */
@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) {
    /* Dark mode defaults */
    --color-bg-primary: #111827;
    /* ... */
  }
}

@media (prefers-color-scheme: light) {
  :root:not([data-theme]) {
    /* Light mode defaults */
    --color-bg-primary: #ffffff;
    /* ... */
  }
}
```
