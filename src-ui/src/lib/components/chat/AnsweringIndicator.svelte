<script lang="ts">
	/**
	 * AnsweringIndicator Component
	 * Displays an animated "Answering..." indicator when the assistant is processing.
	 * Features a subtle scanning light effect to symbolize thinking.
	 */
	import styles from './AnsweringIndicator.module.css';
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Assistant is answering">
	<div class={styles.text}>
		<span class={styles.scanner}>Answering</span>
		<span class={styles.dots}>
			<span class={styles.dot}>.</span>
			<span class={styles.dot}>.</span>
			<span class={styles.dot}>.</span>
		</span>
	</div>
</div>
