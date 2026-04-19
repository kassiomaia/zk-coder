<script lang="ts">
	/**
	 * GroupedToolCallMessage Component
	 *
	 * Renders a collapsed group card for consecutive tool_call messages that
	 * share the same stepNumber. Matches the visual language of ToolCallMessage
	 * exactly. Click the header to expand and reveal the individual cards.
	 */
	import type { ToolGroup } from '$lib/utils/groupToolMessages';
	import { getGroupLabel } from '$lib/utils/groupToolMessages';
	import ToolCallMessage from './ToolCallMessage.svelte';
	import styles from './GroupedToolCallMessage.module.css';
	import { AlertCircle, CheckCircle2, ChevronRight } from 'lucide-svelte';
	import { slide } from 'svelte/transition';
	import { getToolRenderer } from './tool-renderers/registry';

	interface Props {
		group: ToolGroup;
	}

	let { group }: Props = $props();

	let expanded = $state(false);

	function toggleExpand() {
		expanded = !expanded;
	}

	/**
	 * Derive the worst-case status across all group members.
	 * error > pending > success
	 */
	const groupStatus = $derived.by(() => {
		let hasError = false;
		let hasPending = false;
		for (const msg of group.messages) {
			const s = msg.agentData?.toolCallStatus ?? 'pending';
			if (s === 'error') hasError = true;
			if (s === 'pending') hasPending = true;
		}
		if (hasError) return 'error';
		if (hasPending) return 'pending';
		return 'success';
	});

	const label = $derived(getGroupLabel(group.messages));

	// Use the icon from the first message's renderer
	const firstToolName = $derived(group.messages[0]?.agentData?.toolName ?? 'unknown');
	const renderer = $derived(getToolRenderer(firstToolName));
	const ToolIcon = $derived(renderer.icon);
</script>

<div
	class={[styles.group, expanded && styles.expanded, groupStatus === 'error' && styles.hasError]
		.filter(Boolean)
		.join(' ')}
>
	<button
		class={styles.header}
		onclick={toggleExpand}
		type="button"
		aria-label={expanded ? 'Collapse tool group' : 'Expand tool group'}
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
				<span class={styles.actionTitle}>{label}</span>
			</div>
		</div>

		<div class={styles.headerMeta}>
			{#if groupStatus === 'pending'}
				<span class={styles.spinner}></span>
			{:else if groupStatus === 'error'}
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
		<div class={styles.innerList} transition:slide={{ duration: 200 }}>
			{#each group.messages as message (message.id)}
				<div class={styles.innerItem}>
					<ToolCallMessage {message} />
				</div>
			{/each}
		</div>
	{/if}
</div>
