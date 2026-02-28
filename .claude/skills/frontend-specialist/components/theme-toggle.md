# Theme Toggle Component

Button to switch between light and dark themes.

## TypeScript Interface

```typescript
interface ThemeToggleProps {
  /** Current theme */
  theme: 'light' | 'dark' | 'system';

  /** Theme change handler */
  onChange: (theme: 'light' | 'dark' | 'system') => void;

  /** Show system option */
  showSystemOption?: boolean;

  /** Button size */
  size?: 'sm' | 'md' | 'lg';
}
```

---

## Design Tokens

```css
/* Dimensions (circular) */
--toggle-size: 50px;

/* States */
background: var(--color-surface);
opacity: var(--opacity-muted) on normal;
```

---

## Implementation

```svelte
<script lang="ts">
  import styles from './ThemeToggle.module.css';

  interface Props {
    theme: 'light' | 'dark' | 'system';
    onChange: (theme: 'light' | 'dark' | 'system') => void;
    showSystemOption?: boolean;
    size?: 'sm' | 'md' | 'lg';
  }

  let {
    theme,
    onChange,
    showSystemOption = false,
    size = 'md'
  }: Props = $props();

  function toggle() {
    if (theme === 'dark') {
      onChange('light');
    } else {
      onChange('dark');
    }
  }

  let label = $derived(
    theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'
  );
</script>

<button
  class={[styles.toggle, styles[size]].join(' ')}
  onclick={toggle}
  aria-label={label}
  title={label}
>
  {#if theme === 'dark'}
    <SunIcon size={24} />
  {:else}
    <MoonIcon size={24} />
  {/if}
</button>
```

```css
/* ThemeToggle.module.css */
.toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-full);
  cursor: pointer;
  color: var(--color-primary-500);
  opacity: var(--opacity-muted);
  transition: all var(--transition-fast);
}

.toggle:hover {
  opacity: 1;
  background: var(--color-surface-hover);
}

.toggle:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

/* Sizes */
.sm {
  width: 36px;
  height: 36px;
}

.md {
  width: 44px;
  height: 44px;
}

.lg {
  width: 50px;
  height: 50px;
}
```

---

## With Dropdown (All Options)

```svelte
<script lang="ts">
  let isOpen = $state(false);

  function selectTheme(newTheme: 'light' | 'dark' | 'system') {
    onChange(newTheme);
    isOpen = false;
  }
</script>

<div class={styles.container}>
  <button
    class={styles.toggle}
    onclick={() => isOpen = !isOpen}
    aria-expanded={isOpen}
    aria-haspopup="listbox"
  >
    {#if theme === 'dark'}
      <MoonIcon size={20} />
    {:else if theme === 'light'}
      <SunIcon size={20} />
    {:else}
      <MonitorIcon size={20} />
    {/if}
  </button>

  {#if isOpen}
    <div class={styles.menu} role="listbox">
      <button
        class={[styles.option, theme === 'light' && styles.selected].join(' ')}
        onclick={() => selectTheme('light')}
        role="option"
        aria-selected={theme === 'light'}
      >
        <SunIcon size={18} />
        <span>Light</span>
      </button>
      <button
        class={[styles.option, theme === 'dark' && styles.selected].join(' ')}
        onclick={() => selectTheme('dark')}
        role="option"
        aria-selected={theme === 'dark'}
      >
        <MoonIcon size={18} />
        <span>Dark</span>
      </button>
      {#if showSystemOption}
        <button
          class={[styles.option, theme === 'system' && styles.selected].join(' ')}
          onclick={() => selectTheme('system')}
          role="option"
          aria-selected={theme === 'system'}
        >
          <MonitorIcon size={18} />
          <span>System</span>
        </button>
      {/if}
    </div>
  {/if}
</div>
```
