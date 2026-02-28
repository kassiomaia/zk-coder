# Suggestion Cards Component

Quick action cards for prompting the AI assistant.

## TypeScript Interface

```typescript
interface SuggestionCardProps {
  /** Icon to display */
  icon: ComponentType;

  /** Card title */
  title: string;

  /** Card description */
  description: string;

  /** Click handler */
  onclick: () => void;
}

interface SuggestionGridProps {
  /** Array of suggestions */
  suggestions: SuggestionCardProps[];

  /** Number of columns (2-4) */
  columns?: 2 | 3 | 4;
}
```

---

## Design Tokens

```css
/* Card dimensions */
padding: var(--space-4);            /* 16px */
border-radius: var(--radius-xl);    /* 12px */
gap: var(--space-3);                /* 12px (grid) */

/* Icon */
width: 20px;
height: 20px;
color: var(--color-primary-500);
margin-bottom: var(--space-2);

/* Title */
font-size: var(--text-sm);          /* 14px */
font-weight: var(--font-medium);

/* Description */
font-size: var(--text-xs);          /* 12px */
color: var(--color-text-secondary);
```

---

## States

| State | Border | Background |
|-------|--------|------------|
| Normal | `--color-border-primary` | `--color-surface` |
| Hover | `--color-primary-500` | `--color-surface-hover` |
| Focus | `--color-primary-500` (focus ring) | `--color-surface` |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './SuggestionCards.module.css';

  interface CardProps {
    icon: typeof import('svelte').ComponentType;
    title: string;
    description: string;
    onclick: () => void;
  }

  interface Props {
    suggestions: CardProps[];
    columns?: 2 | 3 | 4;
  }

  let { suggestions, columns = 2 }: Props = $props();

  let gridClasses = $derived([styles.grid, styles[`cols-${columns}`]].join(' '));
</script>

<div class={gridClasses}>
  {#each suggestions as suggestion}
    <button
      class={styles.card}
      onclick={suggestion.onclick}
      type="button"
    >
      <div class={styles.icon}>
        <svelte:component this={suggestion.icon} size={20} />
      </div>
      <h3 class={styles.title}>{suggestion.title}</h3>
      <p class={styles.description}>{suggestion.description}</p>
    </button>
  {/each}
</div>
```

```css
/* SuggestionCards.module.css */
.grid {
  display: grid;
  gap: var(--space-3);
  grid-template-columns: repeat(2, 1fr);
}

@media (min-width: 768px) {
  .grid.cols-3 {
    grid-template-columns: repeat(3, 1fr);
  }

  .grid.cols-4 {
    grid-template-columns: repeat(4, 1fr);
  }
}

@media (max-width: 600px) {
  .grid {
    grid-template-columns: 1fr;
  }
}

.card {
  display: flex;
  flex-direction: column;
  padding: var(--space-4);
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-xl);
  cursor: pointer;
  text-align: left;
  transition: all var(--transition-fast);
}

.card:hover {
  border-color: var(--color-primary-500);
  background: var(--color-surface-hover);
}

.card:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}

.icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  margin-bottom: var(--space-2);
  background: rgba(99, 102, 241, 0.1);
  border-radius: var(--radius-md);
  color: var(--color-primary-500);
}

.title {
  font-size: var(--text-sm);
  font-weight: var(--font-medium);
  color: var(--color-text-primary);
  margin: 0 0 var(--space-1);
}

.description {
  font-size: var(--text-xs);
  color: var(--color-text-secondary);
  margin: 0;
  line-height: var(--leading-normal);
}
```

---

## Accessibility

- Use `<button>` for interactive cards
- Include `type="button"`
- Provide focus-visible styles
- Ensure sufficient color contrast for text
