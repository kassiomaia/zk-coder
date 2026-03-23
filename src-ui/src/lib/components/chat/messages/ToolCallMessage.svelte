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
	FileText,
	Edit,
	Trash2,
	Zap,
	Search,
	Globe,
	Wrench,
	Check,
	X,
	ChevronDown,
	ChevronUp
} from 'lucide-svelte';

interface Props {
	message: Message;
}

let { message }: Props = $props();

// Use $derived for reactive values from props
const agentData = $derived(message.agentData);
const toolName = $derived(agentData?.toolName ?? 'unknown');
const toolCallStatus = $derived(agentData?.toolCallStatus ?? 'pending');

let expanded = $state(false);

function toggleExpand() {
	expanded = !expanded;
}

function getToolIcon(name: string): typeof FileText {
	// Return an icon component based on the tool name
	const iconMap: Record<string, any> = {
		read_file: FileText,
		write_file: Edit,
		delete_file: Trash2,
		execute_command: Zap,
		web_search: Search,
		web_fetch: Globe
	};
	return iconMap[name] || Wrench;
}

const ToolIcon = $derived(getToolIcon(toolName));

function formatArgs(args: Record<string, unknown> | undefined): string {
	if (!args) return '';
	return JSON.stringify(args, null, 2);
}

function formatResult(result: unknown): string {
	if (result === null || result === undefined) return '';
	if (typeof result === 'string') return result;
	return JSON.stringify(result, null, 2);
}
</script>

<div class={[styles.toolCall, expanded && styles.expanded].filter(Boolean).join(' ')}>
	<!-- Collapsed header (always visible) -->
	<button
		class={styles.header}
		onclick={toggleExpand}
		type="button"
		aria-label={expanded ? 'Collapse tool call' : 'Expand tool call'}
		aria-expanded={expanded}
	>
		<span class={styles.icon}>
			<ToolIcon size={16} />
		</span>
		<span class={styles.toolName}>{toolName}</span>
		<span class={[styles.status, toolCallStatus === 'error' && styles.error, toolCallStatus === 'pending' && styles.pending].filter(Boolean).join(' ')}>
			{#if toolCallStatus === 'pending'}
				<span class={styles.spinner}></span>
			{:else if toolCallStatus === 'error'}
				<X size={14} />
			{:else}
				<Check size={14} />
			{/if}
		</span>
		<span class={styles.toggle}>
			{#if expanded}
				<ChevronUp size={14} />
			{:else}
				<ChevronDown size={14} />
			{/if}
		</span>
	</button>

	<!-- Expanded content -->
	{#if expanded}
		<div class={styles.content}>
			{#if agentData?.toolArgs && Object.keys(agentData.toolArgs).length > 0}
				<div class={styles.section}>
					<div class={styles.sectionTitle}>Arguments</div>
					<pre class={[styles.code, styles.args].filter(Boolean).join('')}><code>{formatArgs(agentData.toolArgs)}</code></pre>
				</div>
			{/if}

			{#if agentData?.streamingContent || agentData?.toolResult !== undefined}
				<div class={styles.section}>
					<div class={styles.sectionTitle}>Output</div>
					<pre class={[styles.code, styles.output].filter(Boolean).join(' ')}><code>{agentData.streamingContent ?? formatResult(agentData.toolResult)}</code></pre>
				</div>
			{/if}

			{#if agentData?.exitCode !== undefined}
				<div class={styles.section}>
					<div class={styles.exitCode}>
						Exit code: {agentData.exitCode ?? 'unknown'}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
