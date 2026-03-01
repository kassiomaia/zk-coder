<script lang="ts">
	/**
	 * NewThreadButton Component
	 * Button to create a new thread.
	 */
	import styles from './NewThreadButton.module.css';

	interface Props {
		/** Click handler */
		onclick?: () => void;
		/** Disabled state */
		disabled?: boolean;
		/** Compact variant for smaller spaces */
		variant?: 'default' | 'compact';
	}

	let { onclick, disabled = false, variant = 'default' }: Props = $props();
</script>

<button
	class={[styles.button, styles[variant]].filter(Boolean).join(' ')}
	{disabled}
	onclick={onclick}
	aria-label="Create new thread"
	title="New thread"
	type="button"
>
	<svg
		class={styles.icon}
		width="16"
		height="16"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<line x1="12" y1="5" x2="12" y2="19" />
		<line x1="5" y1="12" x2="19" y2="12" />
	</svg>
	{#if variant === 'default'}
		<span class={styles.label}>New</span>
	{/if}
</button>
