<script lang="ts">
	/**
	 * ThinkingMessage Component
	 *
	 * Displays agent thinking/reasoning as a collapsible muted section.
	 * Shows status phrases like "Analyzing...", "Reading files..." during agent execution.
	 * Supports real-time streaming of reasoning content from GLM-5 and similar models.
	 */

	import type { Message } from '$lib/types/message';
	import styles from './ThinkingMessage.module.css';
	import { CheckCircle, ChevronDown, ChevronUp } from 'lucide-svelte';

	interface Props {
		message: Message;
	}

	let { message }: Props = $props();

	// Use $derived for reactive values from props
	const content = $derived(message.content || 'Analyzing request...');
	const reasoning = $derived(message.reasoning);
	const modelName = $derived(message.metadata?.model);
	const isComplete = $derived(message.status === 'sent' || message.content === 'Completed');
	const isStreaming = $derived(message.status === 'sending');

	// Auto-expand when streaming starts - user can still collapse
	let collapsed = $state(false);

	// Auto-expand when we have reasoning content and are streaming
	$effect(() => {
		if (isStreaming && reasoning) {
			collapsed = false;
		}
	});

	function toggleCollapse() {
		collapsed = !collapsed;
	}
</script>

<div
	class={[styles.thinkingMessage, collapsed && styles.collapsed, isComplete && styles.isComplete]
		.filter(Boolean)
		.join(' ')}
>
	<button
		class={styles.header}
		onclick={toggleCollapse}
		type="button"
		aria-label={collapsed ? 'Expand thinking' : 'Collapse thinking'}
		aria-expanded={!collapsed}
	>
		<span class={styles.iconContainer}>
			{#if isComplete}
				<CheckCircle size={12} class={styles.checkIcon} />
			{:else}
				<span class={styles.spinner}></span>
			{/if}
		</span>
		<span class={styles.label}>{isComplete ? 'Thought for a few seconds' : content}</span>

		{#if modelName}
			<span class={styles.modelTag}>{modelName}</span>
		{/if}

		<span class={styles.toggle}>
			{#if collapsed}
				<ChevronDown size={14} />
			{:else}
				<ChevronUp size={14} />
			{/if}
		</span>
	</button>

	{#if !collapsed && (reasoning || isStreaming)}
		<div class={styles.content}>
			{#if reasoning}
				<div class={styles.reasoning}>
					{reasoning}
				</div>
			{:else if isStreaming}
				<div class={styles.streamingIndicator}>
					<span class={styles.streamingText}>Thinking...</span>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	:global(.lucide-cpu) {
		animation: pulse 2s ease-in-out infinite;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
			transform: scale(1);
		}
		50% {
			opacity: 0.7;
			transform: scale(1.1);
		}
	}
</style>
