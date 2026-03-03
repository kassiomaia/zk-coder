<script lang="ts">
	/**
	 * ListItemActions Component
	 * Unified context menu actions for list items (conversations and threads).
	 * Provides consistent styling and behavior across ConversationItem and ThreadItem.
	 */
	import styles from './ListItemActions.module.css';

	interface Props {
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: () => void;
		/** Archive handler */
		onarchive?: () => void;
		/** Resolve/reopen handler (thread-specific) */
		onresolve?: () => void;
		/** Whether the item is resolved (for resolve button state) */
		isResolved?: boolean;
		/** Icon size: 14 for compact view, 16 for standard */
		iconSize?: 14 | 16;
	}

	let { ondelete, onrename, onarchive, onresolve, isResolved = false, iconSize = 16 }: Props = $props();
</script>

<div class={styles.actions} role="toolbar" aria-label="List item actions">
	{#if onresolve}
		<button
			class={[styles.action, styles.resolve].filter(Boolean).join(' ')}
			onclick={(e) => {
				e.stopPropagation();
				onresolve();
			}}
			aria-label={isResolved ? 'Reopen' : 'Resolve'}
			title={isResolved ? 'Reopen' : 'Resolve'}
			type="button"
		>
			{#if isResolved}
				<svg
					width={iconSize}
					height={iconSize}
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
					<path d="M3 3v5h5" />
					<path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
					<path d="M16 21h5v-5" />
				</svg>
			{:else}
				<svg
					width={iconSize}
					height={iconSize}
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
					stroke-linejoin="round"
				>
					<polyline points="20 6 9 17 4 12" />
				</svg>
			{/if}
		</button>
	{/if}

	{#if onrename}
		<button
			class={styles.action}
			onclick={(e) => {
				e.stopPropagation();
				onrename();
			}}
			aria-label="Rename"
			title="Rename"
			type="button"
		>
			<svg
				width={iconSize}
				height={iconSize}
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
				<path d="m15 5 4 4" />
			</svg>
		</button>
	{/if}

	{#if onarchive}
		<button
			class={[styles.action, styles.archive].filter(Boolean).join(' ')}
			onclick={(e) => {
				e.stopPropagation();
				onarchive();
			}}
			aria-label="Archive"
			title="Archive"
			type="button"
		>
			<svg
				width={iconSize}
				height={iconSize}
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
		</button>
	{/if}

	{#if ondelete}
		<button
			class={[styles.action, styles.delete].filter(Boolean).join(' ')}
			onclick={(e) => {
				e.stopPropagation();
				ondelete();
			}}
			aria-label="Delete"
			title="Delete"
			type="button"
		>
			<svg
				width={iconSize}
				height={iconSize}
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<polyline points="3 6 5 6 21 6" />
				<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
			</svg>
		</button>
	{/if}
</div>
