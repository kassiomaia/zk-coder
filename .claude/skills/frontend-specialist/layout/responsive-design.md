# Responsive Design

Guidelines for implementing responsive layouts in the Coder application.

## Breakpoint System

### Tokens

```css
:root {
  --breakpoint-sm: 600px;   /* Mobile */
  --breakpoint-md: 768px;   /* Tablet */
  --breakpoint-lg: 1024px;  /* Desktop */
  --breakpoint-xl: 1280px;  /* Large desktop */
}
```

### Media Query Reference

| Breakpoint | Min Width | Target Devices |
|------------|-----------|----------------|
| `sm` | 600px | Large phones, small tablets |
| `md` | 768px | Tablets, small laptops |
| `lg` | 1024px | Laptops, desktops |
| `xl` | 1280px | Large desktops |

---

## Mobile-First Approach

Write base styles for mobile, then enhance for larger screens:

```css
/* Base (mobile) */
.component {
  padding: var(--space-4);
  font-size: var(--text-sm);
}

/* Tablet and up */
@media (min-width: 768px) {
  .component {
    padding: var(--space-6);
    font-size: var(--text-base);
  }
}

/* Desktop and up */
@media (min-width: 1024px) {
  .component {
    padding: var(--space-8);
  }
}
```

---

## Layout Adaptations

### Sidebar

```css
.sidebar {
  /* Desktop: full sidebar */
  width: var(--sidebar-expanded);
}

/* Tablet: collapsed sidebar */
@media (max-width: 1024px) {
  .sidebar {
    width: var(--sidebar-collapsed);
  }
}

/* Mobile: hidden or bottom nav */
@media (max-width: 600px) {
  .sidebar {
    position: fixed;
    left: -100%;
    transition: left var(--transition-slow);
  }

  .sidebar.open {
    left: 0;
  }
}
```

### Grid Layouts

```css
/* Auto-fit responsive grid */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-4);
}

/* Fixed column counts */
.two-col {
  grid-template-columns: 1fr;
}

.three-col {
  grid-template-columns: 1fr;
}

@media (min-width: 768px) {
  .two-col {
    grid-template-columns: repeat(2, 1fr);
  }

  .three-col {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1024px) {
  .three-col {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### Flex Layouts

```css
/* Stack on mobile, row on desktop */
.flex-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

@media (min-width: 768px) {
  .flex-container {
    flex-direction: row;
    gap: var(--space-6);
  }
}

/* Wrap on small screens */
.flex-wrap {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.flex-wrap > * {
  flex: 1 1 200px; /* Minimum 200px, grow/shrink as needed */
}
```

---

## Component Responsiveness

### Header

```css
.header {
  padding: var(--space-4);
}

.header-title {
  font-size: var(--text-lg);
}

/* Desktop */
@media (min-width: 1024px) {
  .header {
    padding: var(--space-4) var(--space-6);
  }

  .header-title {
    font-size: var(--text-xl);
  }
}
```

### Cards

```css
.card {
  padding: var(--space-4);
}

/* Desktop */
@media (min-width: 768px) {
  .card {
    padding: var(--space-6);
  }
}
```

### Chat Messages

```css
.message {
  max-width: 85%;
}

/* Desktop: narrower for readability */
@media (min-width: 768px) {
  .message {
    max-width: 70%;
  }
}

@media (min-width: 1024px) {
  .message {
    max-width: 60%;
  }
}
```

### Input Box

```css
.input-box {
  padding: var(--space-3);
}

/* Desktop: larger input */
@media (min-width: 768px) {
  .input-box {
    padding: var(--space-4);
  }
}
```

---

## Container Queries (Modern)

For component-level responsiveness:

```css
/* Define container */
.card-container {
  container-type: inline-size;
  container-name: card;
}

/* Query container width */
@container card (min-width: 400px) {
  .card {
    flex-direction: row;
  }

  .card-image {
    width: 200px;
  }
}

@container card (max-width: 399px) {
  .card {
    flex-direction: column;
  }

  .card-image {
    width: 100%;
  }
}
```

---

## Responsive Typography

### Fluid Typography

```css
/* Fluid font size using clamp() */
.heading {
  font-size: clamp(
    var(--text-xl),      /* Minimum */
    5vw,                  /* Preferred */
    var(--text-3xl)       /* Maximum */
  );
}
```

### Line Length

```css
/* Optimal reading width */
.prose {
  max-width: 65ch; /* ~65 characters */
}

/* Responsive max-width */
.content {
  max-width: min(100%, 800px);
}
```

---

## Touch Considerations

### Touch Targets

```css
/* Minimum 44px touch target */
.button {
  min-height: 44px;
  min-width: 44px;
}

/* Increase spacing on touch devices */
@media (hover: none) {
  .list-item {
    padding: var(--space-4);
  }

  .button {
    gap: var(--space-3);
  }
}
```

### Hover States

```css
/* Only show hover on devices that support it */
@media (hover: hover) {
  .card:hover {
    background: var(--color-surface-hover);
  }
}

/* Touch devices: use active state */
@media (hover: none) {
  .card:active {
    background: var(--color-surface-active);
  }
}
```

---

## SvelteKit Responsive Helpers

### Screen Size Store

```typescript
// $lib/stores/screen.ts
import { browser } from '$app/environment';
import { writable, derived } from 'svelte/store';

const breakpoints = {
  sm: 600,
  md: 768,
  lg: 1024,
  xl: 1280
};

function createScreenStore() {
  const width = writable(browser ? window.innerWidth : 1024);
  const height = writable(browser ? window.innerHeight : 768);

  if (browser) {
    window.addEventListener('resize', () => {
      width.set(window.innerWidth);
      height.set(window.innerHeight);
    });
  }

  return {
    width: { subscribe: width.subscribe },
    height: { subscribe: height.subscribe },
    isMobile: derived(width, $w => $w < breakpoints.sm),
    isTablet: derived(width, $w => $w >= breakpoints.sm && $w < breakpoints.lg),
    isDesktop: derived(width, $w => $w >= breakpoints.lg)
  };
}

export const screen = createScreenStore();
```

### Usage in Components

```svelte
<script>
  import { screen } from '$lib/stores/screen';
</script>

{#if $screen.isMobile}
  <MobileNavigation />
{:else}
  <DesktopNavigation />
{/if}
```

---

## Testing Responsive Designs

### Browser DevTools

1. Chrome: F12 → Toggle device toolbar
2. Firefox: F12 → Responsive design mode
3. Safari: Develop → Enter Responsive Design Mode

### Test Breakpoints

Always test at:
- 320px (small mobile)
- 375px (iPhone)
- 600px (breakpoint-sm)
- 768px (breakpoint-md)
- 1024px (breakpoint-lg)
- 1280px (breakpoint-xl)
- 1920px (full HD)
