# File Attachment Component

Display attached files with preview and actions.

## TypeScript Interface

```typescript
interface FileAttachmentProps {
  /** File data */
  file: FileAttachment;

  /** Remove handler */
  onRemove?: () => void;

  /** Click handler */
  onclick?: () => void;

  /** Show preview for images */
  showPreview?: boolean;

  /** Compact mode */
  compact?: boolean;
}

interface FileAttachment {
  id: string;
  name: string;
  type: string;
  size: number;
  preview?: string;
  status?: 'uploading' | 'uploaded' | 'error';
  progress?: number;
}
```

---

## Design Tokens

```css
/* Dimensions */
padding: var(--space-2) var(--space-3);
border-radius: var(--radius-md);

/* Preview image */
width: 40px;
height: 40px;
border-radius: var(--radius-sm);
```

---

## Implementation

```svelte
<script lang="ts">
  import styles from './FileAttachment.module.css';

  interface Props {
    file: FileAttachment;
    onRemove?: () => void;
    onclick?: () => void;
    showPreview?: boolean;
    compact?: boolean;
  }

  let {
    file,
    onRemove,
    onclick,
    showPreview = true,
    compact = false
  }: Props = $props();

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function getFileIcon(type: string): ComponentType {
    if (type.startsWith('image/')) return ImageIcon;
    if (type.startsWith('video/')) return VideoIcon;
    if (type.includes('pdf')) return PdfIcon;
    if (type.includes('zip') || type.includes('archive')) return ArchiveIcon;
    return FileIcon;
  }

  let classes = $derived([
    styles.attachment,
    compact && styles.compact,
    onclick && styles.clickable
  ].filter(Boolean).join(' '));
</script>

<div class={classes} onclick={onclick} role={onclick ? 'button' : undefined}>
  <!-- Preview or Icon -->
  <div class={styles.preview}>
    {#if showPreview && file.preview}
      <img src={file.preview} alt={file.name} class={styles.previewImage} />
    {:else}
      <svelte:component this={getFileIcon(file.type)} size={20} />
    {/if}
  </div>

  <!-- File info -->
  <div class={styles.info}>
    <span class={styles.name}>{file.name}</span>
    {#if !compact}
      <span class={styles.size}>{formatSize(file.size)}</span>
    {/if}
  </div>

  <!-- Status indicator -->
  {#if file.status === 'uploading'}
    <div class={styles.progress}>
      <div class={styles.progressBar} style="width: {file.progress || 0}%"></div>
    </div>
  {:else if file.status === 'error'}
    <span class={styles.errorIcon}>
      <AlertIcon size={16} />
    </span>
  {/if}

  <!-- Remove button -->
  {#if onRemove}
    <button
      class={styles.removeButton}
      onclick={(e) => { e.stopPropagation(); onRemove(); }}
      aria-label="Remove {file.name}"
    >
      <XIcon size={14} />
    </button>
  {/if}
</div>
```

```css
/* FileAttachment.module.css */
.attachment {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  background: var(--color-bg-tertiary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--radius-md);
}

.attachment.clickable {
  cursor: pointer;
  transition: all var(--transition-fast);
}

.attachment.clickable:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-border-secondary);
}

/* Compact mode */
.attachment.compact {
  padding: var(--space-1) var(--space-2);
}

/* Preview */
.preview {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: var(--color-surface);
  border-radius: var(--radius-sm);
  color: var(--color-text-secondary);
}

.compact .preview {
  width: 24px;
  height: 24px;
}

.previewImage {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: inherit;
}

/* Info */
.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.name {
  font-size: var(--text-sm);
  color: var(--color-text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.size {
  font-size: var(--text-xs);
  color: var(--color-text-muted);
}

/* Progress */
.progress {
  flex-shrink: 0;
  width: 60px;
  height: 4px;
  background: var(--color-surface);
  border-radius: var(--radius-full);
  overflow: hidden;
}

.progressBar {
  height: 100%;
  background: var(--color-primary-500);
  transition: width var(--transition-fast);
}

/* Error */
.errorIcon {
  color: var(--color-error);
}

/* Remove button */
.removeButton {
  flex-shrink: 0;
  padding: var(--space-1);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--color-text-muted);
  transition: color var(--transition-fast);
}

.removeButton:hover {
  color: var(--color-error);
}
```
