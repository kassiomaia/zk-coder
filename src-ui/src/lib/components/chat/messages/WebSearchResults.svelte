<script lang="ts">
	/**
	 * Clickable list of web search results. Each row is a `<button>` (not an
	 * `<a>`) so the component controls the open path — URLs go through
	 * `$lib/api/shell.openUrl`, which validates the scheme before calling
	 * `tauri-plugin-opener`. See docs/archive/tools/web_search/plan.md
	 * Appendix B.2.3 for the original specification.
	 */
	import type { WebSearchResult } from '$lib/types/webSearch';
	import { isOpenableUrl, openUrl } from '$lib/api/shell';
	import styles from './WebSearchResults.module.css';

	interface Props {
		results: WebSearchResult[];
		/** Override used by tests to avoid touching the Tauri plugin. */
		onOpen?: (url: string) => void | Promise<void>;
	}

	let { results, onOpen }: Props = $props();

	async function activate(url: string) {
		if (!isOpenableUrl(url)) return;
		try {
			await (onOpen ? onOpen(url) : openUrl(url));
		} catch (err) {
			console.error('web_search open failed', err);
		}
	}
</script>

<ul class={styles.list} role="list" data-testid="web-search-results">
	{#each results as result (result.url)}
		<li>
			<button
				class={styles.row}
				type="button"
				onclick={() => activate(result.url)}
				aria-label="Open {result.title} at {result.source_domain}"
				data-testid="web-search-result"
			>
				<div class={styles.head}>
					<span class={styles.title}>{result.title}</span>
					<span class={styles.ext} aria-hidden="true">↗</span>
				</div>
				<div class={styles.url}>{result.url}</div>
				<div class={styles.snippet}>{result.snippet}</div>
			</button>
		</li>
	{/each}
</ul>
