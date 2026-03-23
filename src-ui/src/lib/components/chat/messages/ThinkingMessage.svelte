<script lang="ts">
/**
 * ThinkingMessage Component
 *
 * Displays agent thinking/reasoning as a collapsible muted section.
 * Shows status phrases like "Analyzing...", "Reading files..." during agent execution.
 */

import type { Message } from '$lib/types/message';
import styles from './ThinkingMessage.module.css';

interface Props {
	message: Message;
}

let { message }: Props = $props();

// Collapsed state
let collapsed = $state(false);

function toggleCollapse() {
	collapsed = !collapsed;
}

// Auto-collapse after completion (if message is done)
$effect(() => {
	// If this is not a transient message, it means the agent is done
	// We could auto-collapse here, but let's keep it manual for now
});
</script>

<div class={[styles.thinkingMessage, collapsed && styles.collapsed].filter(Boolean).join(' ')}>
	<button
		class={styles.header}
		onclick={toggleCollapse}
		type="button"
		aria-label={collapsed ? 'Expand thinking' : 'Collapse thinking'}
		aria-expanded={!collapsed}
	>
		<span class={styles.icon} class:animate={true}>💭</span>
		<span class={styles.label}>Thinking...</span>
		<span class={styles.toggle}>{collapsed ? '+' : '−'}</span>
	</button>

	{#if !collapsed}
		<div class={styles.content}>
			<p class={styles.status}>{message.content || 'Analyzing request...'}</p>
		</div>
	{/if}
</div>

<style>
	.animate {
		display: inline-block;
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
			transform: scale(1.05);
		}
	}
</style>
