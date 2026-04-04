<script lang="ts">
	/**
	 * NewThreadButton Component
	 * Button to create a new thread.
	 */
	import styles from './NewThreadButton.module.css';
	import { PlusIcon } from '$lib/icons';

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
	{onclick}
	aria-label="Create new thread"
	title="New thread"
	type="button"
>
	<PlusIcon class={styles.icon} size={16} />
	{#if variant === 'default'}
		<span class={styles.label}>New</span>
	{/if}
</button>
