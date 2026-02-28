# Header Component

Top navigation bar with context and actions.

## TypeScript Interface

```typescript
interface HeaderProps {
  /** Page/section title */
  title: string;

  /** Optional subtitle/description */
  subtitle?: string;

  /** Breadcrumb items */
  breadcrumbs?: BreadcrumbItem[];

  /** Action buttons */
  actions?: HeaderAction[];

  /** Show back button */
  showBack?: boolean;

  /** Back button handler */
  onBack?: () => void;
}

interface BreadcrumbItem {
  label: string;
  href?: string;
  onclick?: () => void;
}

interface HeaderAction {
  id: string;
  icon?: ComponentType;
  label: string;
  onclick: () => void;
  variant?: 'primary' | 'secondary' | 'ghost';
}
```

---

## Design Tokens

```css
/* Dimensions */
height: var(--header-height);       /* 60px */
padding: var(--space-4) var(--space-6);

/* Typography */
font-size: var(--text-xl);          /* 20px - title */
font-size: var(--text-sm);          /* 14px - subtitle */
```

---

## Implementation

```svelte
<script lang="ts">
  import styles from './Header.module.css';

  interface Props {
    title: string;
    subtitle?: string;
    breadcrumbs?: BreadcrumbItem[];
    actions?: HeaderAction[];
    showBack?: boolean;
    onBack?: () => void;
  }

  let { title, subtitle, breadcrumbs, actions, showBack = false, onBack }: Props = $props();
</script>

<header class={styles.header}>
  <div class={styles.left}>
    {#if showBack && onBack}
      <button class={styles.backButton} onclick={onBack} aria-label="Go back">
        <ChevronIcon direction="left" size={20} />
      </button>
    {/if}

    {#if breadcrumbs && breadcrumbs.length > 0}
      <nav class={styles.breadcrumbs} aria-label="Breadcrumb">
        {#each breadcrumbs as item, i}
          <span class={styles.breadcrumbItem}>
            {#if item.href}
              <a href={item.href}>{item.label}</a>
            {:else if item.onclick}
              <button onclick={item.onclick}>{item.label}</button>
            {:else}
              <span>{item.label}</span>
            {/if}
          </span>
          {#if i < breadcrumbs.length - 1}
            <ChevronIcon direction="right" size={14} class={styles.separator} />
          {/if}
        {/each}
      </nav>
    {/if}

    <div class={styles.titleGroup}>
      <h1 class={styles.title}>{title}</h1>
      {#if subtitle}
        <span class={styles.subtitle}>{subtitle}</span>
      {/if}
    </div>
  </div>

  {#if actions && actions.length > 0}
    <div class={styles.actions}>
      {#each actions as action}
        <Button variant={action.variant || 'secondary'} onclick={action.onclick}>
          {#if action.icon}
            <svelte:component this={action.icon} size={16} />
          {/if}
          {action.label}
        </Button>
      {/each}
    </div>
  {/if}
</header>
```

```css
/* Header.module.css */
.header {
  height: var(--header-height);
  padding: 0 var(--space-6);
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--color-bg-primary);
  border-bottom: 1px solid var(--color-border-primary);
}

.left {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 0; /* Allow truncation */
}

.backButton {
  padding: var(--space-2);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-secondary);
  transition: all var(--transition-fast);
}

.backButton:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.breadcrumbs {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.breadcrumbItem a,
.breadcrumbItem button {
  color: var(--color-text-secondary);
  text-decoration: none;
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
}

.breadcrumbItem a:hover,
.breadcrumbItem button:hover {
  color: var(--color-text-primary);
}

.separator {
  color: var(--color-text-muted);
}

.titleGroup {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.title {
  font-size: var(--text-xl);
  font-weight: var(--font-semibold);
  color: var(--color-text-primary);
  margin: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.subtitle {
  font-size: var(--text-sm);
  color: var(--color-text-secondary);
}

.actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
```
