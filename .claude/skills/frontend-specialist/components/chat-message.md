# Chat Message Component

Message bubbles for user, assistant, and system messages in conversations.

## TypeScript Interface

```typescript
interface ChatMessageProps {
  /** Unique identifier for the message */
  id: string;

  /** Role of the message sender */
  role: 'user' | 'assistant' | 'system';

  /** Message content (may contain markdown) */
  content: string;

  /** Code blocks extracted from content */
  codeBlocks?: CodeBlock[];

  /** Attached files */
  files?: FileAttachment[];

  /** When the message was sent */
  timestamp: Date;

  /** Message status */
  status?: 'sending' | 'sent' | 'error';

  /** Edit handler */
  onEdit?: () => void;

  /** Delete handler */
  onDelete?: () => void;

  /** Retry handler for failed messages */
  onRetry?: () => void;
}

interface CodeBlock {
  id: string;
  language: string;
  code: string;
  filename?: string;
  highlightedLines?: number[];
}

interface FileAttachment {
  id: string;
  name: string;
  type: string;
  size: number;
  preview?: string;
}
```

---

## Design Tokens

```css
/* Dimensions */
max-width: 80%;
padding: var(--space-3) var(--space-4);  /* 12px 16px */
border-radius: var(--radius-xl);          /* 12px */
margin-bottom: var(--space-3);            /* 12px */

/* Typography */
font-size: var(--text-sm);                /* 14px */
line-height: var(--leading-normal);       /* 1.5 */

/* Animation */
animation: messageSlideIn var(--transition-slow) ease-out;
```

---

## Variants

| Variant | Alignment | Background | Text Color | Border |
|---------|-----------|------------|------------|--------|
| User | Right | `--color-primary-500` | `--color-neutral-0` | None |
| Assistant | Left | `--color-surface` | `--color-text-primary` | 1px `--color-border-primary` |
| System | Center | transparent | `--color-accent-violet` | None |

---

## States

| State | Visual Treatment |
|-------|-----------------|
| Default | Normal appearance per variant |
| Sending | Pulsing opacity animation |
| Error | Red border, show retry button |
| Hover | Show edit/delete actions (if handlers provided) |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './ChatMessage.module.css';

  interface Props {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp: Date;
    status?: 'sending' | 'sent' | 'error';
    onEdit?: () => void;
    onDelete?: () => void;
    onRetry?: () => void;
  }

  let {
    id,
    role,
    content,
    timestamp,
    status = 'sent',
    onEdit,
    onDelete,
    onRetry
  }: Props = $props();

  let isHovered = $state(false);

  let messageClasses = $derived(
    [
      styles.message,
      styles[role],
      status === 'sending' && styles.sending,
      status === 'error' && styles.error
    ].filter(Boolean).join(' ')
  );

  function formatTime(date: Date): string {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<article
  class={messageClasses}
  role="article"
  aria-label="{role} message"
>
  {#if role === 'assistant'}
    <span class={styles.sender}>Assistant</span>
  {/if}

  <div class={styles.content}>
    {@html content}
  </div>

  <footer class={styles.footer}>
    <time datetime={timestamp.toISOString()} class={styles.timestamp}>
      {formatTime(timestamp)}
    </time>

    {#if status === 'error' && onRetry}
      <button
        class={styles.retryButton}
        onclick={onRetry}
        aria-label="Retry sending message"
      >
        Retry
      </button>
    {/if}

    {#if isHovered && role !== 'system'}
      <div class={styles.actions}>
        {#if onEdit}
          <button
            class={styles.actionButton}
            onclick={onEdit}
            aria-label="Edit message"
          >
            <EditIcon size={16} />
          </button>
        {/if}
        {#if onDelete}
          <button
            class={styles.actionButton}
            onclick={onDelete}
            aria-label="Delete message"
          >
            <TrashIcon size={16} />
          </button>
        {/if}
      </div>
    {/if}
  </footer>
</article>
```

```css
/* ChatMessage.module.css */
.message {
  max-width: 80%;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-xl);
  margin-bottom: var(--space-3);
  animation: messageSlideIn var(--transition-slow) ease-out;
  position: relative;
}

/* Variants */
.user {
  margin-left: auto;
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
}

.assistant {
  margin-right: auto;
  background: var(--color-surface);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
}

.system {
  margin: var(--space-4) auto;
  max-width: 100%;
  background: transparent;
  color: var(--color-accent-violet);
  text-align: center;
  font-size: var(--text-xs);
  padding: var(--space-2) var(--space-4);
}

/* States */
.sending {
  animation: pulse 1.5s ease-in-out infinite;
}

.error {
  border: 1px solid var(--color-error);
}

/* Elements */
.sender {
  display: block;
  font-size: var(--text-xs);
  font-weight: var(--font-semibold);
  color: var(--color-text-secondary);
  margin-bottom: var(--space-1);
}

.content {
  font-size: var(--text-sm);
  line-height: var(--leading-normal);
}

.footer {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-2);
  font-size: var(--text-2xs);
  color: var(--color-text-muted);
}

.timestamp {
  opacity: var(--opacity-muted);
}

.actions {
  display: flex;
  gap: var(--space-1);
  margin-left: auto;
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.message:hover .actions {
  opacity: 1;
}

.actionButton {
  padding: var(--space-1);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-muted);
  transition: all var(--transition-fast);
}

.actionButton:hover {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.retryButton {
  padding: var(--space-1) var(--space-2);
  font-size: var(--text-2xs);
  color: var(--color-error);
  background: transparent;
  border: 1px solid var(--color-error);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.retryButton:hover {
  background: var(--color-error);
  color: var(--color-neutral-0);
}

/* Animations */
@keyframes messageSlideIn {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
```

---

## Accessibility

### ARIA Attributes

| Element | Attribute | Value |
|---------|-----------|-------|
| Article | `role` | "article" |
| Article | `aria-label` | "{role} message" |
| Time | `datetime` | ISO timestamp |

### Keyboard Support

- Tab to focus message
- Enter/Space to activate action buttons
- Escape to close any open menus

---

## Responsive

```css
/* Mobile: wider messages */
@media (max-width: 600px) {
  .message {
    max-width: 90%;
  }
}

/* Desktop: narrower for readability */
@media (min-width: 1024px) {
  .message {
    max-width: 70%;
  }
}
```
