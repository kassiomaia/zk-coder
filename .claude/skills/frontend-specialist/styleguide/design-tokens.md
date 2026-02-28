# Design Tokens Reference

Complete reference of all CSS custom properties defined in the Coder design system.

**Source**: `src-ui/src/lib/styles/design-system.css`

## Usage Rules

1. **Always use tokens** - Never hardcode values like `#6366f1` or `16px`
2. **Use semantic tokens** - Prefer `--color-text-primary` over `--color-neutral-50`
3. **Theme-aware** - Semantic tokens automatically adapt to light/dark theme

---

## Color Palette

### Primary Colors (Indigo)

| Token | Value | Usage |
|-------|-------|-------|
| `--color-primary-50` | #eef2ff | Light backgrounds, highlights |
| `--color-primary-100` | #e0e7ff | Subtle backgrounds |
| `--color-primary-200` | #c7d2fe | Borders, dividers |
| `--color-primary-300` | #a5b4fc | Disabled accents |
| `--color-primary-400` | #818cf8 | Hover states |
| `--color-primary-500` | #6366f1 | **Primary accent** |
| `--color-primary-600` | #4f46e5 | Active/hover states |
| `--color-primary-700` | #4338ca | Pressed states |
| `--color-primary-800` | #3730a3 | Deep accents |
| `--color-primary-900` | #312e81 | Very dark accents |

### Neutral Colors

| Token | Value | Usage |
|-------|-------|-------|
| `--color-neutral-0` | #ffffff | Pure white |
| `--color-neutral-50` | #f9fafb | Light backgrounds |
| `--color-neutral-100` | #f3f4f6 | Cards, elevated surfaces |
| `--color-neutral-200` | #e5e7eb | Light borders |
| `--color-neutral-300` | #d1d5db | Medium borders |
| `--color-neutral-400` | #9ca3af | Muted text |
| `--color-neutral-500` | #6b7280 | Secondary text |
| `--color-neutral-600` | #4b5563 | Tertiary text |
| `--color-neutral-700` | #374151 | Dark borders |
| `--color-neutral-800` | #1f2937 | Dark surfaces |
| `--color-neutral-900` | #111827 | Dark backgrounds |
| `--color-neutral-950` | #030712 | Very dark backgrounds |

### Accent Colors

| Token | Value | Usage |
|-------|-------|-------|
| `--color-accent-cyan` | #06b6d4 | Info elements |
| `--color-accent-emerald` | #10b981 | Success, positive |
| `--color-accent-violet` | #8b5cf6 | System messages, premium |
| `--color-accent-amber` | #f59e0b | Warning, caution |
| `--color-accent-rose` | #f43f5e | Gradients, accents |

### Semantic Colors

| Token | Value | Usage |
|-------|-------|-------|
| `--color-success` | #10b981 | Success messages, confirmations |
| `--color-warning` | #f59e0b | Warning alerts |
| `--color-error` | #ef4444 | Error states, destructive actions |
| `--color-info` | #06b6d4 | Informational elements |

---

## Theme-Aware Tokens

**IMPORTANT**: Use these tokens for all components. They automatically adapt to the current theme.

### Background Colors

| Token | Dark Mode | Light Mode | Usage |
|-------|-----------|------------|-------|
| `--color-bg-primary` | #111827 | #ffffff | Main background |
| `--color-bg-secondary` | #1f2937 | #f9fafb | Sidebar, panels |
| `--color-bg-tertiary` | #374151 | #f3f4f6 | Cards, elevated surfaces |
| `--color-bg-elevated` | #1f2937 | #ffffff | Modals, dropdowns |

### Text Colors

| Token | Dark Mode | Light Mode | Usage |
|-------|-----------|------------|-------|
| `--color-text-primary` | #e2e8f0 | #111827 | Primary text |
| `--color-text-secondary` | #94a3b8 | #4b5563 | Secondary text |
| `--color-text-tertiary` | #64748b | #6b7280 | Tertiary text |
| `--color-text-muted` | #475569 | #9ca3af | Disabled, hints |

### Surface Colors

| Token | Dark Mode | Light Mode | Usage |
|-------|-----------|------------|-------|
| `--color-surface` | #1f2937 | #ffffff | Component surfaces |
| `--color-surface-hover` | #374151 | #f3f4f6 | Hover states |
| `--color-surface-active` | #4b5563 | #e5e7eb | Active states |

### Border Colors

| Token | Dark Mode | Light Mode | Usage |
|-------|-----------|------------|-------|
| `--color-border-primary` | #374151 | #e5e7eb | Primary borders |
| `--color-border-secondary` | #4b5563 | #d1d5db | Secondary borders |

---

## Typography

### Font Families

| Token | Value | Usage |
|-------|-------|-------|
| `--font-sans` | 'Inter', system-ui | Body text, UI elements |
| `--font-mono` | 'JetBrains Mono', monospace | Code, monospace text |
| `--font-display` | 'Inter', sans-serif | Headings |

### Font Sizes

| Token | Size | Usage |
|-------|------|-------|
| `--text-2xs` | 10px | Captions, metadata, timestamps |
| `--text-xs` | 12px | Small text, labels |
| `--text-sm` | 14px | Body small, code |
| `--text-base` | 16px | Body text |
| `--text-lg` | 18px | Emphasized body |
| `--text-xl` | 20px | H3 headings |
| `--text-2xl` | 24px | H2 headings |
| `--text-3xl` | 28px | H1 headings |
| `--text-4xl` | 36px | Display headings |

### Font Weights

| Token | Value | Usage |
|-------|-------|-------|
| `--font-normal` | 400 | Body text |
| `--font-medium` | 500 | Emphasized text |
| `--font-semibold` | 600 | Headings, labels |
| `--font-bold` | 700 | Strong emphasis |

### Line Heights

| Token | Value | Usage |
|-------|-------|-------|
| `--leading-tight` | 1.25 | Headings |
| `--leading-normal` | 1.5 | Body text |
| `--leading-relaxed` | 1.625 | Long-form text |
| `--leading-code` | 1.4 | Code blocks |

### Letter Spacing

| Token | Value | Usage |
|-------|-------|-------|
| `--tracking-tight` | -0.025em | Tight character spacing |
| `--tracking-normal` | 0 | Normal spacing |
| `--tracking-wide` | 0.025em | Wide spacing |

---

## Spacing

| Token | Size | Usage |
|-------|------|-------|
| `--space-0` | 0 | No spacing |
| `--space-1` | 4px | Tight gaps, icon spacing |
| `--space-2` | 8px | Small gaps, inline elements |
| `--space-3` | 12px | Medium padding, element gaps |
| `--space-4` | 16px | Standard padding |
| `--space-5` | 20px | Medium sections |
| `--space-6` | 24px | Section gaps |
| `--space-8` | 32px | Large gaps |
| `--space-10` | 40px | Major sections |
| `--space-12` | 48px | Page sections |
| `--space-16` | 64px | Large separations |
| `--space-20` | 80px | Extra large separations |
| `--space-24` | 96px | Maximum separations |

---

## Border Radius

| Token | Size | Usage |
|-------|------|-------|
| `--radius-none` | 0 | No rounding |
| `--radius-sm` | 2px | Subtle rounding |
| `--radius-md` | 6px | Buttons, inputs, tags |
| `--radius-lg` | 8px | Cards, panels |
| `--radius-xl` | 12px | Modals, large cards |
| `--radius-2xl` | 16px | Input boxes, large containers |
| `--radius-full` | 9999px | Circular elements, pills |

---

## Shadows

| Token | Value | Usage |
|-------|-------|-------|
| `--shadow-sm` | 0 1px 2px 0 rgb(0 0 0 / 0.05) | Subtle elevation |
| `--shadow-md` | 0 4px 6px -1px rgb(0 0 0 / 0.1) | Dropdowns, popovers |
| `--shadow-lg` | 0 10px 15px -3px rgb(0 0 0 / 0.1) | Modals, dialogs |
| `--shadow-xl` | 0 20px 25px -5px rgb(0 0 0 / 0.1) | High-emphasis elements |

---

## Transitions

| Token | Duration | Usage |
|-------|----------|-------|
| `--transition-fast` | 150ms ease | Micro-interactions, hovers |
| `--transition-normal` | 200ms ease | Standard transitions |
| `--transition-slow` | 300ms ease | Theme changes, sidebar collapse |

---

## Layout

| Token | Value | Usage |
|-------|-------|-------|
| `--sidebar-collapsed` | 70px | Icon-only sidebar width |
| `--sidebar-expanded` | 280px | Full sidebar width |
| `--sidebar-item-size` | 40px | Navigation item size |
| `--header-height` | 60px | Header bar height |
| `--content-max-width` | 1200px | Maximum content width |

---

## Responsive Breakpoints

| Token | Value | Usage |
|-------|-------|-------|
| `--breakpoint-sm` | 600px | Mobile breakpoint |
| `--breakpoint-md` | 768px | Tablet breakpoint |
| `--breakpoint-lg` | 1024px | Desktop breakpoint |
| `--breakpoint-xl` | 1280px | Large desktop breakpoint |

---

## Z-Index Scale

| Token | Value | Usage |
|-------|-------|-------|
| `--z-base` | 0 | Default layer |
| `--z-dropdown` | 100 | Dropdowns, select menus |
| `--z-sticky` | 200 | Sticky headers, toolbars |
| `--z-modal` | 300 | Modals, dialogs |
| `--z-tooltip` | 400 | Tooltips, popovers |
| `--z-toast` | 500 | Toast notifications |

---

## State Opacities

| Token | Value | Usage |
|-------|-------|-------|
| `--opacity-muted` | 0.7 | Inactive icons, secondary elements |
| `--opacity-disabled` | 0.5 | Disabled states |
| `--opacity-subtle` | 0.2 | Subtle backgrounds, overlays |
| `--opacity-hover` | 0.8 | Hover transitions |

---

## Code Display

| Token | Value | Usage |
|-------|-------|-------|
| `--color-code-bg` | #1e1e1e | Code block background |
| `--color-code-text` | #d4d4d4 | Code text color |
| `--color-code-header` | #2d2d2d | Code block header |
| `--color-code-line-number` | #6e7681 | Line numbers |
