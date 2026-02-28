# Color System

Guidelines for using colors in the Coder application.

## Core Principles

1. **Always use theme-aware tokens** - Components must work in both light and dark themes
2. **Never hardcode hex values** - Use CSS custom properties
3. **Maintain semantic meaning** - Use colors consistently (e.g., red always means error/destructive)
4. **Ensure accessibility** - All text must meet WCAG AA contrast requirements

---

## Theme-Aware vs Palette Tokens

### Always Use Theme-Aware (Semantic)

```css
/* CORRECT - Works in both themes */
.card {
  background: var(--color-surface);
  color: var(--color-text-primary);
  border-color: var(--color-border-primary);
}
```

### Avoid Direct Palette Usage

```css
/* INCORRECT - Breaks in one theme */
.card {
  background: #1f2937;        /* Too dark in light mode */
  color: #f9fafb;             /* Too light in light mode */
  border-color: #374151;      /* Invisible in dark mode */
}
```

### When to Use Palette Tokens

Only use palette tokens for:
- **Primary accent colors** (buttons, links, highlights)
- **Status colors** (success, warning, error)
- **Fixed elements** (code blocks use dark background in both themes)

```css
/* Acceptable palette usage */
.button {
  background: var(--color-primary-500);  /* Accent always uses primary */
}

.code-block {
  background: var(--color-code-bg);      /* Code always dark */
}
```

---

## Color Categories

### Background Hierarchy

| Level | Token | Usage |
|-------|-------|-------|
| 1 (Base) | `--color-bg-primary` | Page background, canvas |
| 2 (Elevated) | `--color-bg-secondary` | Sidebar, panels |
| 3 (Higher) | `--color-bg-tertiary` | Cards, elevated surfaces |
| 4 (Highest) | `--color-bg-elevated` | Modals, dropdowns |

**Rule**: Each elevated layer should be lighter (light mode) or darker (dark mode) than the previous.

### Text Hierarchy

| Level | Token | Usage |
|-------|-------|-------|
| Primary | `--color-text-primary` | Headings, body text |
| Secondary | `--color-text-secondary` | Subheadings, descriptions |
| Tertiary | `--color-text-tertiary` | Labels, hints |
| Muted | `--color-text-muted` | Disabled, timestamps |

### Semantic Colors

| Color | Token | Usage |
|-------|-------|-------|
| Success | `--color-success` | Confirmations, positive states |
| Warning | `--color-warning` | Cautions, important notices |
| Error | `--color-error` | Errors, destructive actions |
| Info | `--color-info` | Informational messages |

```css
/* Semantic color usage */
.status-success { color: var(--color-success); }
.status-warning { color: var(--color-warning); }
.status-error { color: var(--color-error); }
.status-info { color: var(--color-info); }
```

### Accent Colors

| Color | Token | Usage |
|-------|-------|-------|
| Primary | `--color-primary-500` | Primary actions, links |
| Violet | `--color-accent-violet` | System messages, premium features |
| Cyan | `--color-accent-cyan` | Special highlights |
| Emerald | `--color-accent-emerald` | Positive accents |
| Amber | `--color-accent-amber` | Warning accents |
| Rose | `--color-accent-rose` | Gradient endpoints |

---

## Interactive States

### Button States

```css
.button {
  /* Default */
  background: var(--color-primary-500);
}

.button:hover {
  background: var(--color-primary-600);
}

.button:active {
  background: var(--color-primary-700);
}

.button:disabled {
  opacity: var(--opacity-disabled);
}
```

### Surface States

```css
.surface {
  background: var(--color-surface);
}

.surface:hover {
  background: var(--color-surface-hover);
}

.surface:active {
  background: var(--color-surface-active);
}
```

---

## Common Patterns

### Card with Border

```css
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  color: var(--color-text-primary);
}
```

### Input Field

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
```

### Link/Interactive Text

```css
.link {
  color: var(--color-primary-500);
}

.link:hover {
  color: var(--color-primary-400);
}
```

### Selection Highlight

```css
::selection {
  background: rgba(99, 102, 241, 0.3);  /* --color-primary-500 with opacity */
  color: var(--color-text-primary);
}
```

---

## Accessibility

### Contrast Requirements (WCAG AA)

| Text Size | Minimum Ratio | Examples |
|-----------|--------------|----------|
| Normal (< 18px) | 4.5:1 | Body text, labels |
| Large (≥ 18px or 14px bold) | 3:1 | Headings, buttons |

### Verified Combinations

These combinations have been verified for WCAG AA compliance:

**Dark Mode:**
- `--color-text-primary` on `--color-bg-primary` ✓
- `--color-text-secondary` on `--color-bg-primary` ✓
- `--color-neutral-0` on `--color-primary-500` ✓

**Light Mode:**
- `--color-text-primary` on `--color-bg-primary` ✓
- `--color-text-secondary` on `--color-bg-primary` ✓
- `--color-neutral-0` on `--color-primary-500` ✓

### Focus Indicators

```css
:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}
```
