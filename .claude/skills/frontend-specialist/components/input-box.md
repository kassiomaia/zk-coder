# Input Box Component

Multi-line text input for composing messages with attachment support.

## TypeScript Interface

```typescript
interface InputBoxProps {
  /** Placeholder text */
  placeholder?: string;

  /** Current input value */
  value: string;

  /** Value change handler */
  onChange: (value: string) => void;

  /** Submit handler */
  onSubmit: () => void;

  /** Disabled state */
  disabled?: boolean;

  /** Loading state */
  loading?: boolean;

  /** Attached files */
  attachments?: FileAttachment[];

  /** File attach handler */
  onAttach?: (files: File[]) => void;

  /** Remove attachment handler */
  onRemoveAttachment?: (id: string) => void;

  /** Maximum character limit */
  maxLength?: number;
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
min-height: 56px;
padding: var(--space-4);            /* 16px */
border-radius: var(--radius-2xl);   /* 16px */
margin-top: var(--space-4);         /* 16px */

/* Typography */
font-size: var(--text-sm);          /* 14px */

/* States */
--opacity-muted: 0.6;               /* Placeholder opacity */
```

---

## Anatomy

```
┌─────────────────────────────────────────────────────────┐
│ [file1.pdf ×] [file2.png ×]                            │  ← Attachments
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Type your message here...                               │  ← Textarea
│                                                         │
├─────────────────────────────────────────────────────────┤
│ [📎] [🎤]                              [Send ●]  500/∞  │  ← Actions
└─────────────────────────────────────────────────────────┘
```

---

## States

| State | Border Color | Background |
|-------|--------------|------------|
| Default | `--color-border-primary` | `--color-surface` |
| Focus | `--color-primary-500` | `--color-surface` |
| Disabled | `--color-border-primary` | `--color-bg-secondary` |
| Error | `--color-error` | `--color-surface` |

---

## Implementation

```svelte
<script lang="ts">
  import styles from './InputBox.module.css';

  interface Props {
    placeholder?: string;
    value: string;
    onChange: (value: string) => void;
    onSubmit: () => void;
    disabled?: boolean;
    loading?: boolean;
    attachments?: FileAttachment[];
    onAttach?: (files: File[]) => void;
    onRemoveAttachment?: (id: string) => void;
    maxLength?: number;
  }

  let {
    placeholder = 'Type your message...',
    value,
    onChange,
    onSubmit,
    disabled = false,
    loading = false,
    attachments = [],
    onAttach,
    onRemoveAttachment,
    maxLength
  }: Props = $props();

  let textareaRef: HTMLTextAreaElement | undefined = $state();
  let fileInputRef: HTMLInputElement | undefined = $state();

  // Auto-resize textarea
  $effect(() => {
    if (textareaRef) {
      textareaRef.style.height = 'auto';
      textareaRef.style.height = `${Math.min(textareaRef.scrollHeight, 200)}px`;
    }
  });

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    onChange(target.value);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (!disabled && !loading && value.trim()) {
        onSubmit();
      }
    }
  }

  function handleFileSelect(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && onAttach) {
      onAttach(Array.from(target.files));
      target.value = '';
    }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  let characterCount = $derived(value.length);
  let isOverLimit = $derived(maxLength !== undefined && characterCount > maxLength);
  let canSubmit = $derived(!disabled && !loading && value.trim() && !isOverLimit);
</script>

<div class={[styles.container, disabled && styles.disabled].filter(Boolean).join(' ')}>
  <!-- Attachments -->
  {#if attachments.length > 0}
    <div class={styles.attachments}>
      {#each attachments as file}
        <div class={styles.attachment}>
          {#if file.preview}
            <img src={file.preview} alt={file.name} class={styles.attachmentPreview} />
          {:else}
            <FileIcon size={16} />
          {/if}
          <span class={styles.attachmentName}>{file.name}</span>
          <span class={styles.attachmentSize}>{formatSize(file.size)}</span>
          <button
            class={styles.attachmentRemove}
            onclick={() => onRemoveAttachment?.(file.id)}
            aria-label="Remove {file.name}"
          >
            <XIcon size={14} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Input area -->
  <div class={styles.inputWrapper}>
    <textarea
      bind:this={textareaRef}
      class={styles.textarea}
      {placeholder}
      {value}
      {disabled}
      oninput={handleInput}
      onkeydown={handleKeydown}
      rows={1}
      aria-label="Message input"
    />
  </div>

  <!-- Actions bar -->
  <div class={styles.actions}>
    <div class={styles.leftActions}>
      {#if onAttach}
        <button
          class={styles.actionButton}
          onclick={() => fileInputRef?.click()}
          disabled={disabled}
          aria-label="Attach file"
        >
          <PaperclipIcon size={20} />
        </button>
        <input
          bind:this={fileInputRef}
          type="file"
          multiple
          accept="image/*,.pdf,.txt,.md"
          onchange={handleFileSelect}
          class={styles.fileInput}
        />
      {/if}
    </div>

    <div class={styles.rightActions}>
      {#if maxLength}
        <span class={[styles.characterCount, isOverLimit && styles.overLimit].filter(Boolean).join(' ')}>
          {characterCount}/{maxLength}
        </span>
      {/if}

      <button
        class={[styles.sendButton, !canSubmit && styles.disabled].filter(Boolean).join(' ')}
        onclick={onSubmit}
        disabled={!canSubmit}
        aria-label="Send message"
      >
        {#if loading}
          <SpinnerIcon size={20} />
        {:else}
          <SendIcon size={20} />
        {/if}
      </button>
    </div>
  </div>
</div>
```

```css
/* InputBox.module.css */
.container {
  background: var(--color-surface);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-2xl);
  padding: var(--space-3);
  transition: border-color var(--transition-fast);
}

.container:focus-within {
  border-color: var(--color-primary-500);
}

.container.disabled {
  opacity: var(--opacity-disabled);
  pointer-events: none;
}

/* Attachments */
.attachments {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}

.attachment {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--color-bg-tertiary);
  border-radius: var(--radius-md);
  font-size: var(--text-xs);
}

.attachmentPreview {
  width: 24px;
  height: 24px;
  object-fit: cover;
  border-radius: var(--radius-sm);
}

.attachmentName {
  color: var(--color-text-primary);
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.attachmentSize {
  color: var(--color-text-muted);
}

.attachmentRemove {
  padding: var(--space-1);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-muted);
  transition: color var(--transition-fast);
}

.attachmentRemove:hover {
  color: var(--color-error);
}

/* Textarea */
.inputWrapper {
  margin-bottom: var(--space-2);
}

.textarea {
  width: 100%;
  min-height: 24px;
  max-height: 200px;
  padding: 0;
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  line-height: var(--leading-normal);
  color: var(--color-text-primary);
  background: transparent;
  border: none;
  outline: none;
  resize: none;
}

.textarea::placeholder {
  color: var(--color-text-muted);
  opacity: var(--opacity-muted);
}

/* Actions */
.actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.leftActions,
.rightActions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.actionButton {
  padding: var(--space-2);
  background: transparent;
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-muted);
  transition: all var(--transition-fast);
}

.actionButton:hover:not(:disabled) {
  background: var(--color-surface-hover);
  color: var(--color-text-primary);
}

.actionButton:disabled {
  opacity: var(--opacity-disabled);
  cursor: not-allowed;
}

.fileInput {
  display: none;
}

/* Character count */
.characterCount {
  font-size: var(--text-xs);
  color: var(--color-text-muted);
}

.characterCount.overLimit {
  color: var(--color-error);
  font-weight: var(--font-medium);
}

/* Send button */
.sendButton {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-primary-500);
  color: var(--color-neutral-0);
  border: none;
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.sendButton:hover:not(.disabled) {
  background: var(--color-primary-600);
}

.sendButton.disabled {
  background: var(--color-text-muted);
  cursor: not-allowed;
}

.sendButton:focus-visible {
  outline: 2px solid var(--color-primary-500);
  outline-offset: 2px;
}
```

---

## Accessibility

### ARIA Attributes

| Element | Attribute | Value |
|---------|-----------|-------|
| Textarea | `aria-label` | "Message input" |
| Send button | `aria-label` | "Send message" |
| Attach button | `aria-label` | "Attach file" |
| Remove button | `aria-label` | "Remove {filename}" |

### Keyboard Support

- Enter to submit (without Shift)
- Shift+Enter for new line
- Tab to navigate actions
- Enter/Space to activate buttons
