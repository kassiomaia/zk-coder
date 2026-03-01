<script lang="ts">
	/**
	 * TypingIndicator Component
	 * Shows an animated indicator when the AI is typing/generating a response.
	 */
	import styles from './TypingIndicator.module.css';

	interface Props {
		/** Optional text to display alongside the animation */
		text?: string;
	}

	let { text = 'Coder is thinking...' }: Props = $props();
</script>

<div class={styles.indicator} role="status" aria-live="polite" aria-label="AI is typing">
	<div class={styles.dots} aria-hidden="true">
		<span class={styles.dot}></span>
		<span class={styles.dot}></span>
		<span class={styles.dot}></span>
	</div>
	{#if text}
		<span class={styles.text}>{text}</span>
	{/if}
</div>
