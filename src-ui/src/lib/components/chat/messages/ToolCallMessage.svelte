<script lang="ts">
/**
 * ToolCallMessage Component
 *
 * Displays a tool execution as a collapsed card (expandable for details).
 * Shows tool name, status indicator, and expandable arguments/output.
 */

import type {Message} from '$lib/types/message';
import styles from './ToolCallMessage.module.css';
import {
    Activity,
    AlertCircle,
    CheckCircle2,
    ChevronDown,
    Code,
    FilePenLine,
    FileSearch,
    FileText,
    Globe,
    Terminal
} from 'lucide-svelte';
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

function toggleExpand() {
	expanded = !expanded;
}

const toolIconMap: Record<string, any> = {
	'fs_read_file': FileText,
	'fs_write_file': FilePenLine,
	'fs_glob': FileSearch,
	'fs_grep': FileSearch,
	'execute_command': Terminal,
	'web_request': Globe,
	'llm_request': Code,
	'read_file': FileText, // compatibility
	'write_file': FilePenLine, // compatibility
	'web_fetch': Globe // compatibility
};

const ToolIcon = $derived(toolIconMap[toolName] || Activity);

function formatJSON(obj: any): string {
	if (!obj) return '';
	try {
		const json = typeof obj === 'string' ? obj : JSON.stringify(obj, null, 2);
		return hljs.highlight(json, { language: 'json' }).value;
	} catch (e) {
		return String(obj);
	}
}

function getPrimaryArg(data: any): string {
	if (!data?.toolArgs) return '';
	const args = data.toolArgs;
	if (args.path) return String(args.path);
	if (args.pattern) return String(args.pattern);
	if (args.command) return String(args.command);
	if (args.url) return String(args.url);
	return '';
}

function formatTime(date: Date): string {
	return date.toLocaleTimeString('en-US', {
		hour12: false,
		hour: '2-digit',
		minute: '2-digit',
		second: '2-digit'
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
		<span class={styles.toolIcon}>
			<ToolIcon size={14} />
		</span>

		<span class={styles.toolName}>{toolName}</span>
		
		{#if getPrimaryArg(agentData)}
			<span class={styles.primaryArg}>{getPrimaryArg(agentData)}</span>
		{/if}

		<div class={styles.headerMeta}>
			{#if toolCallStatus === 'pending'}
				<span class={styles.spinner}></span>
			{:else if isError}
				<span class={[styles.statusIcon, styles.error].join(' ')}>
					<AlertCircle size={14} />
				</span>
			{:else}
				<span class={styles.statusIcon}>
					<CheckCircle2 size={14} />
				</span>
			{/if}
			
			<span class={styles.chevron} class:rotated={expanded}>
				<ChevronDown size={14} />
			</span>
		</div>
	</button>

	{#if expanded}
		<div class={styles.content}>
			{#if agentData?.toolArgs && Object.keys(agentData.toolArgs).length > 0}
				<div class={styles.section}>
					<div class={styles.sectionTitle}>Arguments</div>
					<pre class={styles.jsonBlock}>{@html formatJSON(agentData.toolArgs)}</pre>
				</div>
			{/if}

			{#if agentData?.streamingContent || agentData?.toolResult !== undefined}
				<div class={styles.section}>
					<div class={styles.sectionTitle}>Result</div>
					<pre class={styles.jsonBlock}>{@html formatJSON(agentData.streamingContent ?? agentData.toolResult)}</pre>
				</div>
			{/if}

			{#if agentData?.exitCode !== undefined}
				<div class={styles.section}>
					<div class={styles.exitCode}>
						Exit code: {agentData.exitCode}
					</div>
				</div>
			{/if}
			
			<div class={styles.timestampFooter}>
				Executed at {formatTime(message.timestamp)}
			</div>
		</div>
	{/if}
</div>
