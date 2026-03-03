<script lang="ts">
	/**
	 * TypingIndicator Component
	 * Refined writing-state micro-interaction for assistant responses.
	 *
	 * Features:
	 * - Animated typing dots with soft pulse loop
	 * - Progressive message reveal using controlled opacity shift
	 * - Anticipatory shimmer to signal active generation
	 * - No abrupt layout shifts
	 */
	import styles from './TypingIndicator.module.css';

	interface Props {
		/** Optional text to display alongside the animation */
		text?: string;
		/** Show text label alongside dots */
		showLabel?: boolean;
		/** Variant style: dots, text, or combined */
		variant?: 'dots' | 'text' | 'combined';
	}

	let { text = 'Answering', showLabel = true, variant = 'combined' }: Props = $props();
</script>

<div
	class={[styles.indicator, styles[variant]].filter(Boolean).join(' ')}
	role="status"
	aria-live="polite"
	aria-label="Assistant is typing"
>
	{#if variant === 'dots' || variant === 'combined'}
		<div class={styles.dots} aria-hidden="true">
			<span class={styles.dot}></span>
			<span class={styles.dot}></span>
			<span class={styles.dot}></span>
		</div>
	{/if}

	{#if (variant === 'text' || variant === 'combined') && showLabel}
		<span class={styles.text}>{text}...</span>
	{/if}
</div>
