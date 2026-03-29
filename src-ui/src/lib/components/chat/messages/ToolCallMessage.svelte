<script lang="ts">
/**
 * ToolCallMessage Component
 *
 * Displays a tool execution as a collapsed card (expandable for details).
 * Shows tool name, status indicator, and expandable arguments/output.
 */

  import type { Message } from '$lib/types/message';
  import styles from './ToolCallMessage.module.css';
  import {
    AlertCircle,
    CheckCircle2,
    ChevronRight,
    Code,
    FilePenLine,
    FileSearch,
    FileText,
    Globe,
    Terminal,
    Search,
    Eye,
    PlayCircle
  } from 'lucide-svelte';
  import { slide } from 'svelte/transition';
  import hljs from 'highlight.js';
  import 'highlight.js/styles/github-dark.css';

  interface Props {
    message: Message;
  }

  let { message }: Props = $props();

  // Use $derived for reactive values from props
  const agentData = $derived(message.agentData);
  const toolName = $derived(agentData?.toolName ?? 'unknown');
  const toolCallStatus = $derived(agentData?.toolCallStatus ?? 'pending');
  const isError = $derived(toolCallStatus === 'error');

  let expanded = $state(false);
  let showRaw = $state(false);

  function toggleExpand() {
    expanded = !expanded;
  }

  const toolIconMap: Record<string, any> = {
    'fs_read_file': FileText,
    'fs_write_file': FilePenLine,
    'fs_glob': FileSearch,
    'fs_grep': Search,
    'execute_command': Terminal,
    'web_request': Globe,
    'llm_request': Code,
    'read': FileText,
    'write': FilePenLine,
    'glob': FileSearch,
    'grep': Search,
    'bash': Terminal,
    'webfetch': Globe,
    'skill': PlayCircle,
    'task': Eye,
    'read_file': FileText,
    'write_file': FilePenLine,
    'web_fetch': Globe
  };

  const ToolIcon = $derived(toolIconMap[toolName] || Code);

  function formatJSON(obj: any): string {
    if (!obj) return '';
    try {
      const json = typeof obj === 'string' ? obj : JSON.stringify(obj, null, 2);
      return hljs.highlight(json, { language: 'json' }).value;
    } catch (e) {
      return String(obj);
    }
  }

  function getActionSummary(tool: string, args: any): { title: string; subtitle?: string } {
    if (!args) return { title: tool };

    switch (tool) {
      case 'fs_read_file':
      case 'read_file':
      case 'read':
        return { title: 'Read file', subtitle: args.filePath || args.path };
      case 'fs_write_file':
      case 'write_file':
      case 'write':
        return { title: 'Modified file', subtitle: args.filePath || args.path };
      case 'fs_glob':
      case 'glob':
        return { title: 'Searched files', subtitle: args.pattern };
      case 'fs_grep':
      case 'grep':
        return { title: 'Searched content', subtitle: args.pattern };
      case 'execute_command':
      case 'bash':
        return { title: 'Ran command', subtitle: args.command };
      case 'web_request':
      case 'web_fetch':
      case 'webfetch':
        return { title: 'Fetched URL', subtitle: args.url };
      case 'skill':
        return { title: 'Loaded skill', subtitle: args.name };
      case 'task':
        return { title: 'Spawned agent', subtitle: args.description };
      default:
        return { title: tool, subtitle: Object.values(args)[0] as string };
    }
  }

  function getOutcomeSummary(tool: string, res: any, status: string): string {
    if (status === 'error') return 'Action failed';
    if (status === 'pending') return 'Executing...';
    if (!res) return 'Completed';

    switch (tool) {
      case 'fs_read_file':
      case 'read_file':
      case 'read':
        return `Read ${res.content?.split('\n').length || 0} lines`;
      case 'fs_write_file':
      case 'write_file':
      case 'write':
        return 'Changes applied successfully';
      case 'fs_glob':
      case 'glob':
        const count = Array.isArray(res) ? res.length : (res.paths?.length || 0);
        return `Found ${count} matching files`;
      case 'fs_grep':
      case 'grep':
        const matchCount = Array.isArray(res) ? res.length : (Object.keys(res).length || 0);
        return `Found matches in ${matchCount} files`;
      case 'execute_command':
      case 'bash':
        return res.exit_code === 0 ? 'Command finished successfully' : `Failed with exit code ${res.exit_code}`;
      default:
        return 'Completed';
    }
  }

  const summary = $derived(getActionSummary(toolName, agentData?.toolArgs));
  const outcome = $derived(getOutcomeSummary(toolName, agentData?.toolResult, toolCallStatus));

  // Helper to safely extract error message from toolResult
  function getErrorMessage(result: unknown): string {
    if (result && typeof result === 'object') {
      if ('error' in result && typeof result.error === 'string') {
        return result.error || 'An error occurred';
      }
      if ('message' in result && typeof result.message === 'string') {
        return result.message;
      }
    }
    return 'An unknown error occurred';
  }

  const errorMessage = $derived(agentData?.toolResult !== undefined ? getErrorMessage(agentData.toolResult) : '');

  function formatTime(date: Date): string {
    return date.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit'
    });
  }
</script>

<div class={[styles.toolCall, expanded && styles.expanded, isError && styles.hasError].filter(Boolean).join(' ')}>
  <button
    class={styles.header}
    onclick={toggleExpand}
    type="button"
    aria-label={expanded ? 'Collapse tool call' : 'Expand tool call'}
    aria-expanded={expanded}
  >
    <div class={styles.headerMain}>
      <span class={styles.chevron} class:rotated={expanded}>
        <ChevronRight size={14} />
      </span>

      <div class={styles.toolBadge}>
        <ToolIcon size={12} />
      </div>

      <div class={styles.actionInfo}>
        <span class={styles.actionTitle}>{summary.title}</span>
        {#if summary.subtitle}
          <span class={styles.actionSubtitle}>{summary.subtitle}</span>
        {/if}
      </div>
    </div>

    <div class={styles.headerMeta}>
      <span class={styles.outcomeText}>{outcome}</span>

      {#if toolCallStatus === 'pending'}
        <span class={styles.spinner}></span>
      {:else if isError}
        <span class={[styles.statusIndicator, styles.error].join(' ')}>
          <AlertCircle size={14} />
        </span>
      {:else}
        <span class={[styles.statusIndicator, styles.success].join(' ')}>
          <CheckCircle2 size={14} />
        </span>
      {/if}
    </div>
  </button>

  {#if expanded}
    <div class={styles.content} transition:slide={{ duration: 200 }}>
      <div class={styles.detailsGrid}>
        <div class={styles.detailGroup}>
          <div class={styles.groupLabel}>Parameters</div>
          <div class={styles.parametersList}>
            {#each Object.entries(agentData?.toolArgs || {}) as [key, value]}
              {#if key !== 'path' && key !== 'filePath' && key !== 'pattern' && key !== 'command' && key !== 'url' && key !== 'name'}
                <div class={styles.paramItem}>
                  <span class={styles.paramKey}>{key}:</span>
                  <span class={styles.paramValue}>{typeof value === 'object' ? JSON.stringify(value) : value}</span>
                </div>
              {/if}
            {/each}
            {#if summary.subtitle}
              <div class={styles.paramItem}>
                <span class={styles.paramKey}>Target:</span>
                <span class={styles.paramValue}>{summary.subtitle}</span>
              </div>
            {/if}
          </div>
        </div>

        {#if agentData?.toolResult !== undefined}
          <div class={styles.detailGroup}>
            <div class={styles.groupLabel}>Result Overview</div>
            <div class={styles.resultSummary}>
              {#if isError}
                <div class={styles.errorMessage}>
                  {errorMessage}
                </div>
              {:else}
                <div class={styles.successData}>
                  {outcome}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <div class={styles.rawToggleArea}>
        <button class={styles.rawToggle} onclick={() => (showRaw = !showRaw)}>
          <Code size={12} />
          {showRaw ? 'Hide raw data' : 'View raw JSON'}
        </button>
      </div>

      {#if showRaw}
        <div class={styles.rawView} transition:slide>
          {#if agentData?.toolArgs && Object.keys(agentData.toolArgs).length > 0}
            <div class={styles.section}>
              <div class={styles.sectionLabel}>Arguments</div>
              <pre class={styles.jsonBlock}>{@html formatJSON(agentData.toolArgs)}</pre>
            </div>
          {/if}

          {#if agentData?.streamingContent || agentData?.toolResult !== undefined}
            <div class={styles.section}>
              <div class={styles.sectionLabel}>Result</div>
              <pre class={styles.jsonBlock}>{@html formatJSON(agentData.streamingContent ?? agentData.toolResult)}</pre>
            </div>
          {/if}
        </div>
      {/if}

      <div class={styles.timestampFooter}>
        Executed at {formatTime(message.timestamp)}
      </div>
    </div>
  {/if}
</div>
