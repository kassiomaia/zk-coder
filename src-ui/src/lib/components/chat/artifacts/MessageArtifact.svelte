<script lang="ts">
  /**
   * MessageArtifact Component
   *
   * Displays agent-generated structured outputs as distinct artifacts.
   * Separates deliverables from conversational messages with strong hierarchy.
   *
   * Artifact Types:
   * - table: Structured data with rows and columns
   * - code: Code snippets with syntax highlighting
   * - report: Text-based reports with sections
   * - file: File references with metadata
   * - diagnostic: Error messages and diagnostics
   */
  import type { Snippet } from 'svelte';
  import styles from './MessageArtifact.module.css';
  import { Copy, Download, ExternalLink, ChevronDown, ChevronRight } from 'lucide-svelte';

  export type ArtifactType = 'table' | 'code' | 'report' | 'file' | 'diagnostic';

  export interface ArtifactAction {
    id: string;
    label: string;
    icon: Snippet;
    handler: () => void | Promise<void>;
  }

  export interface ArtifactMetadata {
    title?: string;
    description?: string;
    timestamp?: string;
    source?: string;
    tags?: string[];
  }

  interface Props {
    /** Type of artifact */
    type: ArtifactType;
    /** Artifact content (can be string, object, or snippet) */
    content: Snippet | string | object;
    /** Artifact metadata */
    metadata?: ArtifactMetadata;
    /** Actions available for this artifact */
    actions?: ArtifactAction[];
    /** Whether artifact starts collapsed */
    collapsed?: boolean;
    /** Unique ID for this artifact */
    id?: string;
  }

  let {
    type,
    content,
    metadata = {},
    actions = [],
    collapsed = false,
    id
  }: Props = $props();

  // Internal state - track expanded state (derived from collapsed prop)
  let isExpanded = $derived(!collapsed);

  let isCopying = $state(false);
  let isActionInProgress = $state(false);

  // Derived values
  let hasMetadata = $derived(
    metadata.title || metadata.description || metadata.timestamp || metadata.source
  );

  let hasActions = $derived(actions.length > 0);

  let artifactId = $derived(id || `artifact-${type}-${Math.random().toString(36).slice(2)}`);

  // Get default icon based on type
  function getDefaultIcon(): string {
    switch (type) {
      case 'table':
        return '▦';
      case 'code':
        return '</>';
      case 'report':
        return '📋';
      case 'file':
        return '📄';
      case 'diagnostic':
        return '⚠';
      default:
        return '◆';
    }
  }

  function getTypeLabel(): string {
    switch (type) {
      case 'table':
        return 'Data Table';
      case 'code':
        return 'Code';
      case 'report':
        return 'Report';
      case 'file':
        return 'File';
      case 'diagnostic':
        return 'Diagnostic';
      default:
        return 'Artifact';
    }
  }

  function getTypeClass(): string {
    return styles[type] || styles.default;
  }

  // Handle toggle expand/collapse
  function handleToggle() {
    isExpanded = !isExpanded;
  }

  // Handle copy action
  async function handleCopy() {
    if (isCopying) return;

    isCopying = true;
    try {
      const textContent = typeof content === 'string' ? content : JSON.stringify(content, null, 2);
      await navigator.clipboard.writeText(textContent);
    } catch (error) {
      console.warn('[MessageArtifact] Failed to copy:', error);
    } finally {
      setTimeout(() => {
        isCopying = false;
      }, 1500);
    }
  }

  // Handle artifact action
  async function handleAction(action: ArtifactAction) {
    if (isActionInProgress) return;

    isActionInProgress = true;
    try {
      await action.handler();
    } finally {
      isActionInProgress = false;
    }
  }

  // Format timestamp for display
  function formatTimestamp(timestamp: string): string {
    try {
      const date = new Date(timestamp);
      const now = new Date();
      const diffMs = now.getTime() - date.getTime();
      const diffMins = Math.floor(diffMs / 60000);

      if (diffMins < 1) return 'Just now';
      if (diffMins < 60) return `${diffMins}m ago`;
      if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h ago`;
      return date.toLocaleDateString();
    } catch {
      return timestamp;
    }
  }
</script>

<figure
  class={[styles.artifact, styles[type], isExpanded && styles.expanded].filter(Boolean).join(' ')}
  data-artifact-id={artifactId}
  data-artifact-type={type}
>
  <!-- Artifact Header -->
  <header class={styles.header}>
    <button
      class={styles.headerToggle}
      onclick={handleToggle}
      type="button"
      aria-expanded={isExpanded}
      aria-controls={`${artifactId}-content`}
    >
      <div class={styles.headerLeft}>
        <span class={[styles.icon, getTypeClass()].join(' ')}>{getDefaultIcon()}</span>

        <div class={styles.headerInfo}>
          {#if metadata.title}
            <h3 class={styles.title}>{metadata.title}</h3>
          {:else}
            <h3 class={styles.title}>{getTypeLabel()}</h3>
          {/if}

          <div class={styles.meta}>
            <span class={styles.typeLabel}>{getTypeLabel()}</span>
            {#if metadata.source}
              <span class={styles.separator}>·</span>
              <span class={styles.source}>{metadata.source}</span>
            {/if}
            {#if metadata.timestamp}
              <span class={styles.separator}>·</span>
              <time class={styles.timestamp}>{formatTimestamp(metadata.timestamp)}</time>
            {/if}
          </div>
        </div>
      </div>

      <div class={styles.headerRight}>
        {#if !isExpanded && metadata.description}
          <span class={styles.preview}>{metadata.description}</span>
        {/if}
        <span class={[styles.chevron, isExpanded && styles.chevronExpanded].join(' ')}>
          {#if isExpanded}
            <ChevronDown size={16} strokeWidth={2} />
          {:else}
            <ChevronRight size={16} strokeWidth={2} />
          {/if}
        </span>
      </div>
    </button>
  </header>

  <!-- Artifact Content -->
  {#if isExpanded}
    <div id={`${artifactId}-content`} class={styles.content}>
      {#if metadata.description}
        <p class={styles.description}>{metadata.description}</p>
      {/if}

      <!-- Render content based on type -->
      <div class={styles.contentInner}>
        {#if typeof content === 'string'}
          <pre class={styles.textContent}><code>{content}</code></pre>
        {:else if typeof content === 'function'}
          {@render content()}
        {:else}
          <pre class={styles.textContent}><code>{JSON.stringify(content, null, 2)}</code></pre>
        {/if}
      </div>

      <!-- Artifact Tags -->
      {#if metadata.tags && metadata.tags.length > 0}
        <div class={styles.tags}>
          {#each metadata.tags as tag}
            <span class={styles.tag}>{tag}</span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Artifact Actions Footer -->
  {#if hasActions && isExpanded}
    <footer class={styles.actions}>
      <!-- Copy action (built-in) -->
      <button
        class={[styles.actionBtn, styles.copyBtn].join(' ')}
        onclick={handleCopy}
        disabled={isCopying}
        type="button"
        title="Copy to clipboard"
      >
        {#if isCopying}
          <span class={styles.checkIcon}>✓</span>
          <span>Copied</span>
        {:else}
          <Copy size={14} strokeWidth={2} />
          <span>Copy</span>
        {/if}
      </button>

      <!-- Custom actions -->
      {#each actions as action}
        <button
          class={styles.actionBtn}
          onclick={() => handleAction(action)}
          disabled={isActionInProgress}
          type="button"
          title={action.label}
        >
          {@render action.icon()}
          <span>{action.label}</span>
        </button>
      {/each}
    </footer>
  {/if}
</figure>
