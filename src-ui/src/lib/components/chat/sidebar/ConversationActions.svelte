<script lang="ts">
	/**
	 * ConversationActions Component
	 * Context menu actions for conversation items.
	 */
	import styles from './ConversationActions.module.css';

	interface Props {
		/** Delete handler */
		ondelete?: () => void;
		/** Rename handler */
		onrename?: () => void;
	}

	let { ondelete, onrename }: Props = $props();
</script>

<div class={styles.actions} role="toolbar" aria-label="Conversation actions">
	{#if onrename}
		<button
			class={styles.action}
			onclick={(e) => {
				e.stopPropagation();
				onrename();
			}}
			aria-label="Rename conversation"
			title="Rename"
			type="button"
		>
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
		</button>
	{/if}

	{#if onarchive}
		<button
			class={[styles.action, styles.archive].filter(Boolean).join(' ')}
			onclick={(e) => {
				e.stopPropagation();
				onarchive();
			}}
			aria-label="Archive conversation"
			title="Archive"
			type="button"
		>
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
		</button>
	{/if}

	{#if ondelete}
		<button
			class={[styles.action, styles.delete].filter(Boolean).join(' ')}
			onclick={(e) => {
				e.stopPropagation();
				ondelete();
			}}
			aria-label="Delete conversation"
			title="Delete"
			type="button"
		>
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
				<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
			</svg>
		</button>
	{/if}
</div>
