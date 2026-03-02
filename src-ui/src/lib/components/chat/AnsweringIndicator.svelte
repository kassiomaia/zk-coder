<script lang="ts">
	/**
	 * AnsweringIndicator Component
	 * Displays an animated "Answering..." indicator when the assistant is processing.
	 * Features a scanning light effect to symbolize thinking.
	 */
	import styles from './AnsweringIndicator.module.css';
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Assistant is answering">
	<span class={styles.scanner}>Answering...</span>
</div>
