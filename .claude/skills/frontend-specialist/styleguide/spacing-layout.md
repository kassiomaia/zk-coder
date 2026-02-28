# Spacing & Layout

Guidelines for using spacing and layout tokens in the Coder application.

## Spacing System

### Token Scale

| Token | Size | Usage |
|-------|------|-------|
| `--space-0` | 0 | No spacing |
| `--space-1` | 4px | Tight gaps, icon spacing |
| `--space-2` | 8px | Small gaps, inline spacing |
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

### Usage Patterns

```css
/* Component internal padding */
.card { padding: var(--space-4); }
.button { padding: var(--space-2) var(--space-4); }
.input { padding: var(--space-3) var(--space-4); }

/* Element gaps */
.stack { gap: var(--space-3); }
.inline-group { gap: var(--space-2); }
.grid { gap: var(--space-4); }

/* Section margins */
.section + .section { margin-top: var(--space-8); }
.page-header { margin-bottom: var(--space-6); }

/* Layout margins */
.content { padding: var(--space-6); }
.sidebar { padding: var(--space-5) var(--space-3); }
```

---

## Layout Tokens

### Fixed Dimensions

| Token | Value | Usage |
|-------|-------|-------|
| `--sidebar-collapsed` | 70px | Icon-only sidebar width |
| `--sidebar-expanded` | 280px | Full sidebar width |
| `--sidebar-item-size` | 40px | Navigation item size |
| `--header-height` | 60px | Header bar height |
| `--content-max-width` | 1200px | Maximum content width |

### Responsive Breakpoints

| Token | Value | Usage |
|-------|-------|-------|
| `--breakpoint-sm` | 600px | Mobile breakpoint |
| `--breakpoint-md` | 768px | Tablet breakpoint |
| `--breakpoint-lg` | 1024px | Desktop breakpoint |
| `--breakpoint-xl` | 1280px | Large desktop breakpoint |

---

## Application Layout

### Main Structure

```
┌─────────────────────────────────────────────────────┐
│ Header (60px)                                        │
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│ Sidebar  │            Content Area                  │
│ (70-280) │         (max-width: 1200px)              │
│          │                                          │
│          │                                          │
└──────────┴──────────────────────────────────────────┘
```

### CSS Implementation

```css
.app-layout {
  display: flex;
  min-height: 100vh;
}

.main-content {
  flex: 1;
  min-width: 0; /* Prevent flex overflow */
  max-width: var(--content-max-width);
  padding: var(--space-6);
}

.header {
  height: var(--header-height);
  border-bottom: 1px solid var(--color-border-primary);
}
```

---

## Common Layout Patterns

### Flexbox Patterns

#### Horizontal Stack

```css
.horizontal-stack {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
```

#### Vertical Stack

```css
.vertical-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
```

#### Space Between

```css
.space-between {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
```

#### Centered Content

```css
.centered {
  display: flex;
  justify-content: center;
  align-items: center;
}
```

### Grid Patterns

#### Auto-Fit Grid

```css
.auto-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: var(--space-4);
}
```

#### Fixed Columns

```css
.two-column-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-3);
}

@media (min-width: 768px) {
  .three-column-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

#### Sidebar Layout

```css
.sidebar-layout {
  display: grid;
  grid-template-columns: var(--sidebar-expanded) 1fr;
  min-height: 100vh;
}

.sidebar-layout.collapsed {
  grid-template-columns: var(--sidebar-collapsed) 1fr;
}
```

---

## Component Spacing

### Button Sizing

```css
/* Small */
.button-sm {
  padding: var(--space-1) var(--space-3);
  font-size: var(--text-xs);
  gap: var(--space-1);
}

/* Medium (default) */
.button-md {
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-sm);
  gap: var(--space-2);
}

/* Large */
.button-lg {
  padding: var(--space-3) var(--space-6);
  font-size: var(--text-base);
  gap: var(--space-2);
}
```

### Card Spacing

```css
.card {
  padding: var(--space-4);
  gap: var(--space-3);
}

.card-header {
  padding-bottom: var(--space-3);
  margin-bottom: var(--space-3);
  border-bottom: 1px solid var(--color-border-primary);
}

.card-footer {
  padding-top: var(--space-3);
  margin-top: var(--space-3);
  border-top: 1px solid var(--color-border-primary);
}
```

### Form Spacing

```css
.form-group {
  margin-bottom: var(--space-4);
}

.form-label {
  margin-bottom: var(--space-2);
}

.form-help {
  margin-top: var(--space-1);
  font-size: var(--text-xs);
  color: var(--color-text-muted);
}
```

### List Spacing

```css
.list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.list-item {
  padding: var(--space-3) var(--space-4);
}
```

---

## Responsive Patterns

### Mobile-First Approach

```css
/* Base styles (mobile) */
.component {
  padding: var(--space-4);
}

/* Tablet and up */
@media (min-width: 768px) {
  .component {
    padding: var(--space-6);
  }
}

/* Desktop and up */
@media (min-width: 1024px) {
  .component {
    padding: var(--space-8);
  }
}
```

### Container Queries (Modern)

```css
.card-container {
  container-type: inline-size;
}

@container (min-width: 400px) {
  .card {
    flex-direction: row;
  }
}
```

### Sidebar Responsive Behavior

```css
.sidebar {
  width: var(--sidebar-expanded);
  transition: width var(--transition-slow);
}

/* Tablet: collapse to icons */
@media (max-width: 1024px) {
  .sidebar {
    width: var(--sidebar-collapsed);
  }
}

/* Mobile: hide completely or bottom nav */
@media (max-width: 600px) {
  .sidebar {
    display: none;
  }
}
```

---

## Z-Index Management

### Layer Order

```css
/* Base content */
.content { z-index: var(--z-base); }        /* 0 */

/* Dropdowns and popovers */
.dropdown { z-index: var(--z-dropdown); }   /* 100 */

/* Sticky elements */
.sticky-header { z-index: var(--z-sticky); } /* 200 */

/* Modals and dialogs */
.modal { z-index: var(--z-modal); }         /* 300 */

/* Tooltips */
.tooltip { z-index: var(--z-tooltip); }     /* 400 */

/* Toast notifications */
.toast { z-index: var(--z-toast); }         /* 500 */
```

### Stacking Context

```css
/* Create new stacking context when needed */
.new-stacking-context {
  position: relative;
  z-index: 0;
}
```
