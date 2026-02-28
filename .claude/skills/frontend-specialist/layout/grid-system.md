# Grid System

Grid and flexbox patterns for the Coder application.

## Grid Tokens

The application uses CSS Grid and Flexbox for layouts. There are no specific grid tokens - use spacing tokens for gaps.

```css
/* Common gap values */
gap: var(--space-2);   /* 8px */
gap: var(--space-3);   /* 12px */
gap: var(--space-4);   /* 16px */
gap: var(--space-6);   /* 24px */
```

---

## CSS Grid Patterns

### Auto-Fit Grid

```css
/* Responsive grid that adjusts columns automatically */
.auto-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-4);
}
```

### Fixed Column Grids

```css
/* 2 columns */
.grid-2 {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-4);
}

/* 3 columns */
.grid-3 {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-4);
}

/* 4 columns */
.grid-4 {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
}
```

### Responsive Columns

```css
.grid-responsive {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--space-4);
}

@media (min-width: 600px) {
  .grid-responsive {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1024px) {
  .grid-responsive {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### Sidebar Layout

```css
/* Main app layout with sidebar */
.app-grid {
  display: grid;
  grid-template-columns: var(--sidebar-expanded) 1fr;
  min-height: 100vh;
}

.app-grid.collapsed {
  grid-template-columns: var(--sidebar-collapsed) 1fr;
}

/* With header */
.app-grid-with-header {
  display: grid;
  grid-template-columns: var(--sidebar-expanded) 1fr;
  grid-template-rows: var(--header-height) 1fr;
  grid-template-areas:
    "sidebar header"
    "sidebar main";
  min-height: 100vh;
}
```

### Asymmetric Layouts

```css
/* 2/3 - 1/3 split */
.split-2-3 {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: var(--space-6);
}

/* 1/4 - 3/4 split */
.split-1-4 {
  display: grid;
  grid-template-columns: 1fr 3fr;
  gap: var(--space-6);
}

/* Fixed sidebar with fluid content */
.sidebar-layout {
  display: grid;
  grid-template-columns: 280px 1fr;
  gap: var(--space-6);
}
```

### Span Columns

```css
/* Item spanning 2 columns */
.span-2 {
  grid-column: span 2;
}

/* Item spanning full width */
.span-full {
  grid-column: 1 / -1;
}
```

---

## Flexbox Patterns

### Horizontal Stack

```css
/* Basic horizontal layout */
.h-stack {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

/* With wrapping */
.h-stack-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}
```

### Vertical Stack

```css
/* Basic vertical layout */
.v-stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

/* Stretch children */
.v-stretch {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.v-stretch > * {
  flex: 1;
}
```

### Space Distribution

```css
/* Space between items */
.space-between {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

/* Space around items */
.space-around {
  display: flex;
  justify-content: space-around;
  align-items: center;
}

/* Evenly distributed */
.space-evenly {
  display: flex;
  justify-content: space-evenly;
  align-items: center;
}
```

### Centering

```css
/* Center horizontally and vertically */
.center {
  display: flex;
  justify-content: center;
  align-items: center;
}

/* Center horizontally only */
.center-x {
  display: flex;
  justify-content: center;
}

/* Center vertically only */
.center-y {
  display: flex;
  align-items: center;
}
```

### Flex Grow/Shrink

```css
/* Item grows to fill space */
.grow {
  flex: 1;
}

/* Item doesn't shrink */
.no-shrink {
  flex-shrink: 0;
}

/* Fixed width item */
.fixed-width {
  flex: 0 0 200px;
}
```

---

## Common Layout Components

### Card Grid

```css
.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: var(--space-4);
}

.card {
  padding: var(--space-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-lg);
}
```

### List with Actions

```css
.list-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
}

.list-item:hover {
  background: var(--color-surface-hover);
}

.list-item-content {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex: 1;
  min-width: 0; /* Allow truncation */
}

.list-item-actions {
  display: flex;
  gap: var(--space-2);
  flex-shrink: 0;
}
```

### Form Layout

```css
.form-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: var(--space-4);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.form-row {
  display: flex;
  gap: var(--space-4);
}

.form-row > * {
  flex: 1;
}
```

### Split View

```css
.split-view {
  display: grid;
  grid-template-columns: 1fr 1fr;
  height: 100%;
}

.split-pane {
  overflow: auto;
  padding: var(--space-4);
}

/* With divider */
.split-view-with-divider {
  display: grid;
  grid-template-columns: 1fr 1px 1fr;
  height: 100%;
}

.split-divider {
  background: var(--color-border-primary);
}
```

### Sticky Footer

```css
.page-with-footer {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.page-content {
  flex: 1;
}

.page-footer {
  flex-shrink: 0;
  padding: var(--space-4);
  border-top: 1px solid var(--color-border-primary);
}
```

---

## Grid Template Areas

### Named Areas

```css
.dashboard {
  display: grid;
  grid-template-columns: 1fr 300px;
  grid-template-rows: auto 1fr auto;
  grid-template-areas:
    "header header"
    "main sidebar"
    "footer footer";
  min-height: 100vh;
  gap: var(--space-4);
}

.dashboard-header { grid-area: header; }
.dashboard-main { grid-area: main; }
.dashboard-sidebar { grid-area: sidebar; }
.dashboard-footer { grid-area: footer; }
```

### Responsive Areas

```css
.dashboard {
  grid-template-columns: 1fr;
  grid-template-areas:
    "header"
    "main"
    "sidebar"
    "footer";
}

@media (min-width: 1024px) {
  .dashboard {
    grid-template-columns: 1fr 300px;
    grid-template-areas:
      "header header"
      "main sidebar"
      "footer footer";
  }
}
```

---

## Gap vs Margin

Use `gap` for spacing between grid/flex children:

```css
/* PREFERRED: Using gap */
.grid {
  display: grid;
  gap: var(--space-4);
}

/* AVOID: Using margins */
.grid-legacy {
  display: grid;
}

.grid-legacy > * {
  margin: var(--space-2);
}
```

**Benefits of `gap`:**
- No margin on first/last items
- Consistent spacing
- Works with `flex-wrap`
- Easier to reason about
