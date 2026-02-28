# Code Block Component

Syntax-highlighted code display with copy functionality.

## TypeScript Interface

```typescript
interface CodeBlockProps {
  /** Programming language for syntax highlighting */
  language: string;

  /** Code content */
  code: string;

  /** Optional filename */
  filename?: string;

  /** Show line numbers */
  showLineNumbers?: boolean;

  /** Lines to highlight */
  highlightedLines?: number[];

  /** Diff information */
  diff?: {
    type: 'addition' | 'deletion' | 'modification';
    additions: number;
    deletions: number;
  };

  /** Copy callback */
  onCopy?: () => void;

  /** Edit callback */
  onEdit?: () => void;
}
```

---

## Design Tokens

```css
/* Code colors (fixed, not theme-aware) */
--color-code-bg: #1e1e1e;
--color-code-text: #d4d4d4;
--color-code-header: #2d2d2d;
--color-code-line-number: #6e7681;

/* Dimensions */
padding: var(--space-3);           /* 12px */
border-radius: var(--radius-lg);   /* 8px */
margin-top: var(--space-2);        /* 8px */

/* Typography */
font-family: var(--font-mono);
font-size: var(--text-xs);         /* 12px */
line-height: var(--leading-code);  /* 1.4 */
```

---

## Anatomy

```
┌─────────────────────────────────────────────────┐
│ TypeScript                      [Copy] [Edit]  │  ← Header
├─────────────────────────────────────────────────┤
│  1 │ const greeting = "Hello, World!";         │
│  2 │ console.log(greeting);                    │  ← Code with line numbers
│  3 │ return greeting;                          │
└─────────────────────────────────────────────────┘
```

---

## States

| Element | Normal | Hover |
|---------|--------|-------|
| Copy Button | `--color-text-secondary` | `--color-neutral-0` |
| Edit Button | `--color-text-secondary` | `--color-neutral-0` |
| Code Background | `--color-code-bg` | (no change) |
| Highlighted Line | `primary-500` at 0.1 opacity | (no change) |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './CodeBlock.module.css';

  interface Props {
    language: string;
    code: string;
    filename?: string;
    showLineNumbers?: boolean;
    highlightedLines?: number[];
    onCopy?: () => void;
    onEdit?: () => void;
  }

  let {
    language,
    code,
    filename,
    showLineNumbers = true,
    highlightedLines = [],
    onCopy,
    onEdit
  }: Props = $props();

  let copied = $state(false);

  const lines = code.split('\n');

  async function handleCopy() {
    await navigator.clipboard.writeText(code);
    copied = true;
    onCopy?.();
    setTimeout(() => copied = false, 2000);
  }

  function isHighlighted(lineNum: number): boolean {
    return highlightedLines.includes(lineNum);
  }
</script>

<div class={styles.container}>
  <header class={styles.header}>
    <div class={styles.headerLeft}>
      <span class={styles.language}>{language}</span>
      {#if filename}
        <span class={styles.filename}>{filename}</span>
      {/if}
    </div>
    <div class={styles.actions}>
      {#if onEdit}
        <button
          class={styles.actionButton}
          onclick={onEdit}
          aria-label="Edit code"
        >
          <EditIcon size={14} />
        </button>
      {/if}
      <button
        class={styles.actionButton}
        onclick={handleCopy}
        aria-label="Copy code"
      >
        {#if copied}
          <CheckIcon size={14} />
        {:else}
          <CopyIcon size={14} />
        {/if}
        <span>{copied ? 'Copied!' : 'Copy'}</span>
      </button>
    </div>
  </header>

  <pre class={styles.code}><code>{#each lines as line, i}
    <div
      class={[
        styles.line,
        isHighlighted(i + 1) && styles.highlighted
      ].filter(Boolean).join(' ')}
    >
      {#if showLineNumbers}
        <span class={styles.lineNumber} aria-hidden="true">{i + 1}</span>
      {/if}
      <span class={styles.lineContent}>{line}</span>
    </div>{/each}
  </code></pre>
</div>
```

```css
/* CodeBlock.module.css */
.container {
  background: var(--color-code-bg);
  border-radius: var(--radius-lg);
  margin: var(--space-2) 0;
  overflow: hidden;
  font-family: var(--font-mono);
}

/* Header */
.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-2) var(--space-3);
  background: var(--color-code-header);
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.headerLeft {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.language {
  font-size: var(--text-2xs);
  font-weight: var(--font-medium);
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.filename {
  font-size: var(--text-2xs);
  color: var(--color-code-line-number);
}

.actions {
  display: flex;
  gap: var(--space-2);
}

.actionButton {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-2xs);
  color: var(--color-text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: color var(--transition-fast);
}

.actionButton:hover {
  color: var(--color-neutral-0);
}

/* Code */
.code {
  padding: var(--space-3);
  margin: 0;
  overflow-x: auto;
}

.line {
  display: flex;
  min-height: 1.4em;
}

.line.highlighted {
  background: rgba(99, 102, 241, 0.15);
  border-left: 2px solid var(--color-primary-500);
  margin-left: calc(var(--space-3) * -1);
  padding-left: calc(var(--space-3) - 2px);
}

.lineNumber {
  min-width: 3ch;
  padding-right: var(--space-3);
  color: var(--color-code-line-number);
  user-select: none;
  text-align: right;
}

.lineContent {
  flex: 1;
  white-space: pre;
}

/* Diff indicators */
.line.addition {
  background: rgba(16, 185, 129, 0.15);
}

.line.addition .lineNumber::before {
  content: '+';
  color: var(--color-success);
}

.line.deletion {
  background: rgba(239, 68, 68, 0.15);
}

.line.deletion .lineNumber::before {
  content: '-';
  color: var(--color-error);
}
```

---

## Accessibility

### ARIA Attributes

| Element | Attribute | Value |
|---------|-----------|-------|
| Pre | `role` | "code" (via `<code>`) |
| Line numbers | `aria-hidden` | "true" |
| Copy button | `aria-label` | "Copy code" |

### Keyboard Support

- Tab to focus copy button
- Enter/Space to activate copy
- Arrow keys to scroll code horizontally

---

## Syntax Highlighting

For production syntax highlighting, use a library like Prism or Shiki:

```svelte
<script>
  import { highlightCode } from '$lib/utils/syntax';

  $effect(() => {
    if (container) {
      highlightCode(container, language);
    }
  });
</script>
```

---

## Responsive

```css
/* Ensure code doesn't overflow container */
.container {
  max-width: 100%;
}

.code {
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}

/* Mobile: smaller font */
@media (max-width: 600px) {
  .lineContent {
    font-size: 11px;
  }
}
```
