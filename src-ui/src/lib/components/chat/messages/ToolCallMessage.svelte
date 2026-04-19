<script lang="ts">
	/**
	 * ToolCallMessage Component
	 *
	 * Displays a tool execution as a collapsed card (expandable for details).
	 * Shows tool name, status indicator, and expandable arguments/output.
	 */

	import type { Message } from '$lib/types/message';
	import styles from './ToolCallMessage.module.css';
	import { AlertCircle, CheckCircle2, ChevronRight, Code } from 'lucide-svelte';
	import { slide } from 'svelte/transition';
	import hljs from 'highlight.js';
	import 'highlight.js/styles/github-dark.css';
	import { getToolRenderer } from './tool-renderers/registry';
	import type { ToolCallStatus } from './tool-renderers/types';
	import { agentStore } from '$lib/stores/agent';

	interface Props {
		message: Message;
	}

	let { message }: Props = $props();

	// Use $derived for reactive values from props
	const agentData = $derived(message.agentData);
	const toolName = $derived(agentData?.toolName ?? 'unknown');
	const toolCallStatus = $derived<ToolCallStatus>(
		(agentData?.toolCallStatus ?? 'pending') as ToolCallStatus
	);
	const isError = $derived(toolCallStatus === 'error');

	let expanded = $state(false);
	let showRaw = $state(false);

	function toggleExpand() {
		expanded = !expanded;
	}

	// Resolve the per-tool renderer from the static registry. Unknown tool
	// names fall through to the `defaultRenderer`, which surfaces the raw
	// tool name as the title so the user still has a clue what ran.
	const renderer = $derived(getToolRenderer(toolName));
	const ToolIcon = $derived(renderer.icon);

	function formatJSON(obj: any): string {
		if (!obj) return '';
		try {
			const json = typeof obj === 'string' ? obj : JSON.stringify(obj, null, 2);
			return hljs.highlight(json, { language: 'json' }).value;
		} catch (e) {
			return String(obj);
		}
	}

	const summary = $derived(renderer.getSummary(agentData?.toolArgs, toolName));
	const outcome = $derived(renderer.getOutcome(agentData?.toolResult, toolCallStatus));

	// Resolve the TouchedFile for write operations by looking up the session's
	// filesTouched array using the sessionId and the path from toolArgs.
	const touchedFile = $derived(
		(() => {
			const sessionId = agentData?.sessionId ?? '';
			const filePath =
				(agentData?.toolArgs as { filePath?: string; path?: string } | undefined)?.filePath ??
				(agentData?.toolArgs as { filePath?: string; path?: string } | undefined)?.path;
			if (!sessionId || !filePath) return undefined;
			const files = $agentStore.sessions.get(sessionId)?.filesTouched;
			return files?.find((tf) => tf.path === filePath);
		})()
	);

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

	const errorMessage = $derived(
		agentData?.toolResult !== undefined ? getErrorMessage(agentData.toolResult) : ''
	);

	function formatTime(date: Date): string {
		return date.toLocaleTimeString('en-US', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<div
	class={[styles.toolCall, expanded && styles.expanded, isError && styles.hasError]
		.filter(Boolean)
		.join(' ')}
>
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
									<span class={styles.paramValue}
										>{typeof value === 'object' ? JSON.stringify(value) : value}</span
									>
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

				{#if renderer.BodyComponent}
					{@const BodyComponent = renderer.BodyComponent}
					<div class={styles.detailGroup}>
						<div class={styles.groupLabel}>Result Overview</div>
						<BodyComponent
							args={agentData?.toolArgs}
							result={agentData?.toolResult}
							status={toolCallStatus}
							streamingContent={agentData?.streamingContent}
							{touchedFile}
						/>
					</div>
				{:else if agentData?.toolResult !== undefined}
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
							<pre class={styles.jsonBlock}>{@html formatJSON(
									agentData.streamingContent ?? agentData.toolResult
								)}</pre>
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
