<script lang="ts">
	/**
	 * ConversationActions Component
	 * Context menu actions for conversation items.
	 */
	import styles from './ConversationActions.module.css';
	import IconButton from '$lib/components/shared/IconButton.svelte';

	interface Props {
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: () => void;
		/** Archive handler */
		onarchive?: () => void;
	}

	let { ondelete, onrename, onarchive }: Props = $props();
</script>

<div class={styles.actions} role="toolbar" aria-label="Conversation actions">
	{#if onrename}
		<IconButton
			label="Rename conversation"
			tooltip="Rename"
			tooltipPosition="left"
			variant="ghost"
			size="xs"
			onclick={(e) => {
				e.stopPropagation();
				onrename();
			}}
		>
			{#snippet icon()}
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
					<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
				</svg>
			{/snippet}
		</IconButton>
	{/if}

	{#if onarchive}
		<IconButton
			label="Archive conversation"
			tooltip="Archive"
			tooltipPosition="left"
			variant="ghost"
			intent="default"
			size="xs"
			onclick={(e) => {
				e.stopPropagation();
				onarchive();
			}}
		>
			{#snippet icon()}
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="21 8 21 21 3 21 3 8" />
					<rect x="1" y="3" width="22" height="5" />
					<line x1="10" y1="12" x2="14" y2="12" />
				</svg>
			{/snippet}
		</IconButton>
	{/if}

	{#if ondelete}
		<IconButton
			label="Delete conversation"
			tooltip="Delete"
			tooltipPosition="left"
			variant="ghost"
			intent="danger"
			size="xs"
			onclick={(e) => {
				e.stopPropagation();
				ondelete();
			}}
		>
			{#snippet icon()}
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="3 6 5 6 21 6" />
					<path
						d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
					/>
				</svg>
			{/snippet}
		</IconButton>
	{/if}
</div>
