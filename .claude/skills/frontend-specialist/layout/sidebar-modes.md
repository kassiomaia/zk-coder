# Sidebar Modes

Collapsible sidebar implementation with expanded and collapsed states.

## Overview

The sidebar supports two modes:
- **Expanded** (280px): Full sidebar with icons and labels
- **Collapsed** (70px): Icon-only sidebar for more screen space

---

## Design Tokens

```css
:root {
  --sidebar-collapsed: 70px;
  --sidebar-expanded: 280px;
  --sidebar-item-size: 40px;
  --sidebar-icon-size: 24px;
}
```

---

## Anatomy

### Expanded Mode (280px)

```
┌────────────────────────┐
│   ┌────────────────┐   │
│   │     Logo       │   │  ← 40px × 40px
│   └────────────────┘   │
│                        │
│   ┌────────────────┐   │
│   │ 🔵 Chat        │   │  ← Active nav item
│   └────────────────┘   │
│                        │
│   ┌────────────────┐   │
│   │ 📁 Files       │   │
│   └────────────────┘   │
│                        │
│   ┌────────────────┐   │
│   │ ⚙️ Settings    │   │
│   └────────────────┘   │
│                        │
│        ...             │
│                        │
│   ┌────────────────┐   │
│   │  << Collapse   │   │  ← Toggle button
│   └────────────────┘   │
└────────────────────────┘
```

### Collapsed Mode (70px)

```
┌──────────┐
│  ┌────┐  │
│  │ 🔵 │  │  ← Logo (centered)
│  └────┘  │
│          │
│  ┌────┐  │
│  │ 💬 │  │  ← Active (highlighted)
│  └────┘  │
│          │
│  ┌────┐  │
│  │ 📁 │  │
│  └────┘  │
│          │
│  ┌────┐  │
│  │ ⚙️ │  │
│  └────┘  │
│          │
│   ...    │
│          │
│  ┌────┐  │
│  │ >> │  │  ← Toggle button
│  └────┘  │
└──────────┘
```

---

## Component States

| State | Opacity | Background | Notes |
|-------|---------|------------|-------|
| Normal | 0.7 | transparent | Default state |
| Hover | 1.0 | --color-surface-hover | Mouse over |
| Active | 1.0 | primary with 0.2 opacity | Selected item |
| Disabled | 0.5 | transparent | Non-interactive |

---

## Svelte Implementation

### Sidebar Component

```svelte
<script lang="ts">
  import styles from './Sidebar.module.css';

  interface NavItem {
    id: string;
    icon: typeof import('svelte').ComponentType;
    label: string;
    badge?: string | number;
    href?: string;
    onclick?: () => void;
  }

  interface Props {
    collapsed?: boolean;
    activeItem?: string;
    items: NavItem[];
    onToggle?: () => void;
  }

  let {
    collapsed = false,
    activeItem,
    items,
    onToggle
  }: Props = $props();

  let sidebarClasses = $derived(
    [styles.sidebar, collapsed && styles.collapsed]
      .filter(Boolean)
      .join(' ')
  );
</script>

<aside class={sidebarClasses} role="navigation" aria-label="Main navigation">
  <!-- Logo -->
  <div class={styles.logo}>
    <LogoIcon size={32} />
  </div>

  <!-- Navigation -->
  <nav class={styles.nav}>
    {#each items as item}
      <a
        class={[
          styles.navItem,
          activeItem === item.id && styles.active
        ].filter(Boolean).join(' ')}
        href={item.href}
        onclick={item.onclick}
        aria-current={activeItem === item.id ? 'page' : undefined}
        title={collapsed ? item.label : undefined}
      >
        <svelte:component this={item.icon} size={24} />
        {#if !collapsed}
          <span class={styles.label}>{item.label}</span>
        {/if}
        {#if item.badge && !collapsed}
          <span class={styles.badge}>{item.badge}</span>
        {/if}
      </a>
    {/each}
  </nav>

  <!-- Toggle Button -->
  <button
    class={styles.toggle}
    onclick={onToggle}
    aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
  >
    <ChevronIcon direction={collapsed ? 'right' : 'left'} size={20} />
  </button>
</aside>
```

### CSS Module

```css
/* Sidebar.module.css */
.sidebar {
  width: var(--sidebar-expanded);
  height: 100vh;
  padding: var(--space-5) 0;
  background: var(--color-bg-secondary);
  border-right: 1px solid var(--color-border-primary);
  display: flex;
  flex-direction: column;
  transition: width var(--transition-slow);
}

.sidebar.collapsed {
  width: var(--sidebar-collapsed);
}

/* Logo */
.logo {
  width: var(--sidebar-item-size);
  height: var(--sidebar-item-size);
  margin: 0 auto var(--space-8);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-lg);
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
}

/* Navigation */
.nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: 0 var(--space-2);
  overflow-y: auto;
}

/* Navigation Item */
.navItem {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  height: var(--sidebar-item-size);
  padding: 0 var(--space-3);
  border-radius: var(--radius-md);
  background: transparent;
  border: none;
  cursor: pointer;
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
  text-decoration: none;
  color: inherit;
}

.collapsed .navItem {
  justify-content: center;
  padding: 0;
}

.navItem:hover {
  opacity: 1;
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.navItem.active {
  opacity: 1;
  background: rgba(99, 102, 241, var(--opacity-subtle));
  color: var(--color-primary-500);
}

/* Label */
.label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Badge */
.badge {
  margin-left: auto;
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border-radius: var(--radius-full);
}

/* Toggle Button */
.toggle {
  margin: var(--space-4) auto;
  padding: var(--space-2);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  cursor: pointer;
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
  display: flex;
  align-items: center;
  justify-content: center;
}

.toggle:hover {
  opacity: 1;
  background: var(--color-surface-hover);
}

.toggle:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}
```

---

## Tooltips in Collapsed Mode

When collapsed, show tooltips on hover:

```svelte
<script>
  import { tooltip } from '$lib/actions/tooltip';
</script>

<a
  class={styles.navItem}
  use:tooltip={{ content: item.label, position: 'right' }}
>
  <svelte:component this={item.icon} />
</a>
```

```css
/* Tooltip styles */
.tooltip {
  position: absolute;
  left: calc(100% + var(--space-2));
  padding: var(--space-2) var(--space-3);
  background: var(--color-bg-elevated);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
  font-size: var(--text-xs);
  white-space: nowrap;
  z-index: var(--z-tooltip);
  box-shadow: var(--shadow-md);
}
```

---

## Keyboard Navigation

```svelte
<script>
  function handleKeydown(e: KeyboardEvent, index: number) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusItem(index + 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusItem(index - 1);
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        items[index].onclick?.();
        break;
      case 'Home':
        e.preventDefault();
        focusItem(0);
        break;
      case 'End':
        e.preventDefault();
        focusItem(items.length - 1);
        break;
    }
  }
</script>

<nav role="menubar" aria-label="Main navigation">
  {#each items as item, index}
    <a
      role="menuitem"
      tabindex={index === 0 ? 0 : -1}
      onkeydown={(e) => handleKeydown(e, index)}
    >
      <!-- ... -->
    </a>
  {/each}
</nav>
```

---

## Responsive Behavior

```css
/* Desktop: Expanded by default */
.sidebar {
  width: var(--sidebar-expanded);
}

/* Tablet: Collapsed by default */
@media (max-width: 1024px) {
  .sidebar {
    width: var(--sidebar-collapsed);
  }

  .sidebar:not(.collapsed) .label,
  .sidebar:not(.collapsed) .badge {
    display: none;
  }
}

/* Mobile: Hidden, show bottom nav instead */
@media (max-width: 600px) {
  .sidebar {
    display: none;
  }
}
```

---

## Persistence

Store sidebar state in preferences:

```typescript
// $lib/stores/preferences.ts
import { persisted } from '$lib/stores/persisted';

export const preferences = persisted('coder-preferences', {
  sidebarCollapsed: false,
  // ...
});

// In component
import { preferences } from '$lib/stores/preferences';

let collapsed = $state($preferences.sidebarCollapsed);

function toggle() {
  collapsed = !collapsed;
  preferences.update(p => ({ ...p, sidebarCollapsed: collapsed }));
}
```
