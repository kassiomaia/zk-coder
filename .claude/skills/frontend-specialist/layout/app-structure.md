# Application Structure

Main application layout architecture for the Coder AI Code Agent.

## Layout Overview

```
┌─────────────────────────────────────────────────────────────┐
│ Header (60px)                                                │
│ - App title, context, actions                                │
│ - Theme toggle, user menu                                    │
├──────────┬──────────────────────────────────────────────────┤
│          │                                                   │
│ Sidebar  │                 Main Content                      │
│ 70-280px │              (max-width: 1200px)                  │
│          │                                                   │
│ - Logo  │  ┌─────────────────────────────────────────────┐   │
│ - Nav   │  │ Greeting Section                             │   │
│ - Items │  │ - Personalized greeting                      │   │
│          │  │ - Subtitle/description                       │   │
│          │  └─────────────────────────────────────────────┘   │
│          │                                                   │
│          │  ┌─────────────────────────────────────────────┐   │
│          │  │ Suggestion Cards / Content Area              │   │
│          │  │ - Quick actions or main content              │   │
│          │  └─────────────────────────────────────────────┘   │
│          │                                                   │
│          │  ┌─────────────────────────────────────────────┐   │
│          │  │ Input Box                                    │   │
│          │  │ - Message input                              │   │
│          │  │ - Send button, attachments                   │   │
│          │  └─────────────────────────────────────────────┘   │
│          │                                                   │
└──────────┴──────────────────────────────────────────────────┘
```

---

## Layout Tokens

```css
:root {
  /* Fixed dimensions */
  --sidebar-collapsed: 70px;
  --sidebar-expanded: 280px;
  --sidebar-item-size: 40px;
  --header-height: 60px;
  --content-max-width: 1200px;

  /* Responsive breakpoints */
  --breakpoint-sm: 600px;
  --breakpoint-md: 768px;
  --breakpoint-lg: 1024px;
  --breakpoint-xl: 1280px;
}
```

---

## Application Shell

### Svelte Implementation

```svelte
<!-- +layout.svelte -->
<script lang="ts">
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Header from '$lib/components/Header.svelte';
  import styles from './layout.module.css';

  let { children } = $props();
</script>

<div class={styles.app}>
  <Sidebar />
  <div class={styles.main}>
    <Header />
    <main class={styles.content}>
      {@render children()}
    </main>
  </div>
</div>
```

```css
/* layout.module.css */
.app {
  display: flex;
  min-height: 100vh;
  background: var(--color-bg-primary);
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.content {
  flex: 1;
  display: flex;
  flex-direction: column;
  max-width: var(--content-max-width);
  padding: var(--space-6);
  overflow-y: auto;
}
```

---

## Responsive Behavior

### Breakpoint Strategy

| Breakpoint | Sidebar | Header | Content |
|------------|---------|--------|---------|
| < 600px (Mobile) | Hidden or bottom nav | Compact | Full width |
| 600-1024px (Tablet) | Collapsed (70px) | Full | Fluid |
| > 1024px (Desktop) | Expanded (280px) | Full | Centered |

### CSS Implementation

```css
/* Desktop (default) */
.sidebar {
  width: var(--sidebar-expanded);
}

/* Tablet */
@media (max-width: 1024px) {
  .sidebar {
    width: var(--sidebar-collapsed);
  }

  .sidebar-label {
    display: none;
  }
}

/* Mobile */
@media (max-width: 600px) {
  .sidebar {
    display: none;
  }

  /* Alternative: Bottom navigation */
  .bottom-nav {
    display: flex;
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 60px;
    background: var(--color-bg-secondary);
    border-top: 1px solid var(--color-border-primary);
  }
}
```

---

## Z-Index Layers

```css
/* Layer order for overlapping elements */
.sidebar      { z-index: var(--z-sticky); }    /* 200 */
.header       { z-index: var(--z-sticky); }    /* 200 */
.dropdown     { z-index: var(--z-dropdown); }  /* 100 */
.modal        { z-index: var(--z-modal); }     /* 300 */
.tooltip      { z-index: var(--z-tooltip); }   /* 400 */
.toast        { z-index: var(--z-toast); }     /* 500 */
```

---

## Grid System

### Main Layout Grid

```css
.app-grid {
  display: grid;
  grid-template-columns: var(--sidebar-expanded) 1fr;
  grid-template-rows: var(--header-height) 1fr;
  min-height: 100vh;
}

.app-grid.collapsed {
  grid-template-columns: var(--sidebar-collapsed) 1fr;
}

/* Grid areas */
.app-grid {
  grid-template-areas:
    "sidebar header"
    "sidebar main";
}

.sidebar { grid-area: sidebar; }
.header { grid-area: header; }
.main { grid-area: main; }
```

### Content Grid

```css
.content-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-4);
}

/* Fixed column layouts */
.two-columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-6);
}

@media (max-width: 768px) {
  .two-columns {
    grid-template-columns: 1fr;
  }
}
```

---

## Scroll Behavior

### Main Content Scroll

```css
.content {
  overflow-y: auto;
  scroll-behavior: smooth;
}

/* Respect reduced motion */
@media (prefers-reduced-motion: reduce) {
  .content {
    scroll-behavior: auto;
  }
}
```

### Sticky Elements

```css
.header {
  position: sticky;
  top: 0;
  z-index: var(--z-sticky);
  background: var(--color-bg-primary);
  border-bottom: 1px solid var(--color-border-primary);
}

/* Input box stuck to bottom */
.input-container {
  position: sticky;
  bottom: 0;
  background: var(--color-bg-primary);
  padding: var(--space-4) 0;
  border-top: 1px solid var(--color-border-primary);
}
```

---

## Safe Areas (Desktop)

For desktop applications, account for window chrome:

```css
/* Safe padding for window controls */
.title-bar-area {
  height: 32px; /* Windows/Linux title bar */
  -webkit-app-region: drag; /* Tauri: draggable area */
}

/* Traffic light area (macOS) */
.traffic-lights {
  padding-left: 80px; /* Space for close/minimize/maximize */
}
```

---

## Common Layout Patterns

### Chat Layout

```css
.chat-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.chat-input {
  flex-shrink: 0;
  padding: var(--space-4);
  border-top: 1px solid var(--color-border-primary);
}
```

### Split View

```css
.split-view {
  display: grid;
  grid-template-columns: 1fr 1fr;
  height: 100%;
  gap: 1px;
  background: var(--color-border-primary);
}

.split-pane {
  background: var(--color-bg-primary);
  overflow: auto;
}

/* Resizable split */
.split-view {
  position: relative;
}

.split-handle {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: col-resize;
  background: transparent;
  z-index: 10;
}

.split-handle:hover {
  background: var(--color-primary-500);
}
```

### Empty State

```css
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: var(--space-8);
  text-align: center;
}

.empty-state-icon {
  width: 64px;
  height: 64px;
  margin-bottom: var(--space-4);
  opacity: var(--opacity-muted);
}

.empty-state-title {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  margin-bottom: var(--space-2);
}

.empty-state-description {
  color: var(--color-text-secondary);
  max-width: 400px;
}
```
