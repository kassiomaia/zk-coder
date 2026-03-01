<script lang="ts">
	/**
	 * Skeleton Component
	 * Loading placeholder with shimmer animation.
	 * Used to improve perceived performance during content loading.
	 */
	import styles from './Skeleton.module.css';

	type SkeletonVariant = 'text' | 'circle' | 'rect' | 'message' | 'conversationItem' | 'card';
	type SkeletonSize = 'sm' | 'md' | 'lg';

	interface Props {
		/** Visual variant of the skeleton */
		variant?: SkeletonVariant;
		/** Size preset */
		size?: SkeletonSize;
		/** Custom width */
		width?: string;
		/** Custom height */
		height?: string;
		/** Additional CSS class */
		class?: string;
		/** Number of items to render (for text variant) */
		count?: number;
		/** Enable animation */
		animate?: boolean;
	}

	let {
		variant = 'text',
		size = 'md',
		width,
		height,
		class: className = '',
		count = 1,
		animate = true
	}: Props = $props();

	let containerClasses = $derived(
		[styles.skeleton, styles[variant], styles[size], !animate && styles.noAnimation, className]
			.filter(Boolean)
			.join(' ')
	);

	let style = $derived(() => {
		const styles: string[] = [];
		if (width) styles.push(`width: ${width}`);
		if (height) styles.push(`height: ${height}`);
		return styles.join('; ');
	});
</script>

{#if variant === 'message'}
	<div class={styles.message}>
		<div class={styles.messageRow}>
			<div class={[styles.skeleton, styles.circle, styles.sm].join(' ')}></div>
			<div class={[styles.skeleton, styles.messageLine, styles.long].join(' ')}></div>
		</div>
		<div class={styles.messageLine}>
			<div class={[styles.skeleton, styles.messageLine, styles.medium].join(' ')}></div>
		</div>
		<div class={styles.messageLine}>
			<div class={[styles.skeleton, styles.messageLine, styles.short].join(' ')}></div>
		</div>
	</div>
{:else if variant === 'conversationItem'}
	<div class={styles.conversationItem}>
		<div class={[styles.skeleton, styles.conversationTitle].join(' ')}></div>
		<div class={styles.conversationMeta}>
			<div class={[styles.skeleton, styles.conversationMetaText].join(' ')}></div>
			<div class={[styles.skeleton, styles.conversationMetaText].join(' ')}></div>
		</div>
	</div>
{:else if variant === 'card'}
	<div class={styles.card}>
		<div class={styles.cardHeader}>
			<div class={[styles.skeleton, styles.cardTitle].join(' ')}></div>
			<div class={[styles.skeleton, styles.circle, styles.sm].join(' ')}></div>
		</div>
		<div class={styles.cardContent}>
			{#each Array(count) as _}
				<div class={[styles.skeleton, styles.cardLine, styles.long].join(' ')}></div>
			{/each}
		</div>
	</div>
{:else}
	{#each Array(count) as _}
		<div class={containerClasses} style={style()}></div>
	{/each}
{/if}
