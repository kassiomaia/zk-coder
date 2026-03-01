<script lang="ts">
	/**
	 * ThreadBreadcrumbs Component
	 * Navigation breadcrumbs for thread hierarchy.
	 */
	import styles from './ThreadBreadcrumbs.module.css';

	interface BreadcrumbItem {
		id: string;
		title: string;
	}

	interface Props {
		/** Breadcrumb path items */
		path: BreadcrumbItem[];
		/** Handler when a breadcrumb is clicked */
		onNavigate?: (id: string) => void;
		/** Maximum visible items before truncation */
		maxVisible?: number;
	}

	let { path, onNavigate, maxVisible = 3 }: Props = $props();

	// Determine which items to show
	let visibleItems = $derived(() => {
		if (path.length <= maxVisible) {
			return { start: path, end: [] };
		}

		const startCount = 1;
		const endCount = maxVisible - 1;

		return {
			start: path.slice(0, startCount),
			end: path.slice(-endCount)
		};
	})();

	let showEllipsis = $derived(path.length > maxVisible);
</script>

{#if path.length > 0}
	<nav class={styles.breadcrumbs} aria-label="Thread navigation">
		<ol class={styles.list}>
			{#each visibleItems.start as item, index}
				<li class={styles.item}>
					<button
						class={styles.link}
						onclick={() => onNavigate?.(item.id)}
						type="button"
						disabled={!onNavigate}
					>
						{item.title}
					</button>
					<svg
						class={styles.separator}
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<polyline points="9 18 15 12 9 6" />
					</svg>
				</li>
			{/each}

			{#if showEllipsis}
				<li class={styles.item}>
					<span class={styles.ellipsis} aria-label="More items">...</span>
					<svg
						class={styles.separator}
						width="14"
						height="14"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						aria-hidden="true"
					>
						<polyline points="9 18 15 12 9 6" />
					</svg>
				</li>
			{/if}

			{#each visibleItems.end as item, index}
				<li class={styles.item}>
					{#if index < visibleItems.end.length - 1}
						<button
							class={styles.link}
							onclick={() => onNavigate?.(item.id)}
							type="button"
							disabled={!onNavigate}
						>
							{item.title}
						</button>
						<svg
							class={styles.separator}
							width="14"
							height="14"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
							aria-hidden="true"
						>
							<polyline points="9 18 15 12 9 6" />
						</svg>
					{:else}
						<span class={styles.current} aria-current="page">{item.title}</span>
					{/if}
				</li>
			{/each}
		</ol>
	</nav>
{/if}
