<script lang="ts">
	/**
	 * StatusBadge Component
	 * Unified status indicator for threads and conversations.
	 * Single source of truth for status styling across the application.
	 */
	import styles from './StatusBadge.module.css';

	export type StatusVariant = 'open' | 'resolved' | 'pending';

	interface Props {
		/** Status variant to display */
		variant: StatusVariant;
		/** Optional label override (defaults to variant name) */
		label?: string;
		/** Show checkmark icon for resolved status */
		showIcon?: boolean;
		/** Compact mode for use in lists */
		compact?: boolean;
	}

	let { variant, label, showIcon = true, compact = false }: Props = $props();

	let displayLabel = $derived(label || variant.charAt(0).toUpperCase() + variant.slice(1));
	let containerClasses = $derived(
		[styles.badge, styles[variant], compact && styles.compact].filter(Boolean).join(' ')
	);
</script>

<span class={containerClasses} aria-label="Status: {displayLabel}">
	{#if variant === 'resolved' && showIcon}
		<svg
			class={styles.icon}
			width={compact ? 10 : 12}
			height={compact ? 10 : 12}
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
	{#if !compact}
		<span class={styles.text}>{displayLabel}</span>
	{/if}
</span>
