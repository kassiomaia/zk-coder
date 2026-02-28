# Sidebar Component

Vertical navigation bar with collapsible modes.

## TypeScript Interface

```typescript
interface SidebarProps {
  /** Whether sidebar is collapsed */
  collapsed: boolean;

  /** Toggle handler */
  onToggle: () => void;

  /** Currently active section */
  activeSection: string;

  /** Navigation items */
  sections: SidebarSection[];
}

interface SidebarSection {
  id: string;
  icon: ComponentType;
  label: string;
  badge?: string | number;
  href?: string;
  onclick?: () => void;
  disabled?: boolean;
}
```

---

## Design Tokens

```css
/* Widths */
--sidebar-collapsed: 70px;
--sidebar-expanded: 280px;
--sidebar-item-size: 40px;

/* Spacing */
padding: var(--space-5) 0;
margin: var(--space-6) auto;

/* States */
--opacity-muted: 0.7;
```

---

## States

| State | Opacity | Background |
|-------|---------|------------|
| Normal | 0.7 | transparent |
| Hover | 1.0 | `--color-surface-hover` |
| Active | 1.0 | `primary-500` at 0.2 opacity |
| Disabled | 0.5 | transparent |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './Sidebar.module.css';

  interface Props {
    collapsed: boolean;
    onToggle: () => void;
    activeSection: string;
    sections: SidebarSection[];
  }

  let { collapsed, onToggle, activeSection, sections }: Props = $props();

  let sidebarClasses = $derived(
    [styles.sidebar, collapsed && styles.collapsed].filter(Boolean).join(' ')
  );
</script>

<aside class={sidebarClasses} role="navigation" aria-label="Main navigation">
  <div class={styles.logo}>
    <LogoIcon size={32} />
  </div>

  <nav class={styles.nav}>
    {#each sections as section}
      <a
        class={[
          styles.navItem,
          activeSection === section.id && styles.active,
          section.disabled && styles.disabled
        ].filter(Boolean).join(' ')}
        href={section.href}
        onclick={section.onclick}
        aria-current={activeSection === section.id ? 'page' : undefined}
        title={collapsed ? section.label : undefined}
      >
        <svelte:component this={section.icon} size={24} />
        {#if !collapsed}
          <span class={styles.label}>{section.label}</span>
        {/if}
        {#if section.badge && !collapsed}
          <span class={styles.badge}>{section.badge}</span>
        {/if}
      </a>
    {/each}
  </nav>

  <button
    class={styles.toggle}
    onclick={onToggle}
    aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
  >
    <ChevronIcon direction={collapsed ? 'right' : 'left'} size={20} />
  </button>
</aside>
```

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

.nav {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: 0 var(--space-2);
  overflow-y: auto;
}

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
  color: var(--color-text-primary);
}

.collapsed .navItem {
  justify-content: center;
  padding: 0;
}

.navItem:hover:not(.disabled) {
  opacity: 1;
  background: var(--color-surface-hover);
}

.navItem.active {
  opacity: 1;
  background: rgba(99, 102, 241, var(--opacity-subtle));
  color: var(--color-primary-500);
}

.navItem.disabled {
  opacity: var(--opacity-disabled);
  cursor: not-allowed;
}

.label {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  margin-left: auto;
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border-radius: var(--radius-full);
}

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
