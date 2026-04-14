<script lang="ts">
	/**
	 * Custom expanded body for `search_web_search` tool calls.
	 *
	 * Routes between four visual states driven by `(status, result)`:
	 * - pending           → nothing (the header spinner carries the signal)
	 * - error + not_configured → actionable banner with "Open settings →"
	 * - error + any other code → generic error banner
	 * - success + count 0      → neutral "No results" banner
	 * - success + filtered_by_policy true → policy banner + result list
	 * - success + cached true             → cached banner + result list
	 * - success + count > 0               → result list
	 *
	 * See docs/archive/tools/web_search/plan.md Appendix B.2.2 for the
	 * original specification of these state transitions.
	 */
	import WebSearchResults from '../WebSearchResults.svelte';
	import {
		isWebSearchError,
		type WebSearchArgs,
		type WebSearchErrorBody,
		type WebSearchOutput
	} from '$lib/types/webSearch';
	import type { ToolCallStatus } from './types';
	import styles from './WebSearchBody.module.css';

	interface Props {
		args: WebSearchArgs | undefined;
		result: WebSearchOutput | WebSearchErrorBody | undefined;
		status: ToolCallStatus;
	}

	let { args: _args, result, status }: Props = $props();

	const isError = $derived(status === 'error' || isWebSearchError(result));
	const errorCode = $derived(
		isWebSearchError(result) ? result.error_code : undefined
	);
	const errorMessage = $derived(
		isWebSearchError(result) ? result.error : undefined
	);
	const success = $derived<WebSearchOutput | undefined>(
		!isError && result && 'count' in (result as object)
			? (result as WebSearchOutput)
			: undefined
	);

	function openSettings() {
		// Placeholder hook — the router-specific helper lands alongside
		// Settings navigation. Until then we log so QA can confirm the
		// click fired.
		console.info('[web_search] open settings requested');
	}
</script>

<div class={styles.wrap}>
	{#if status === 'pending'}
		<!-- Header spinner communicates state; body intentionally empty. -->
	{:else if isError && errorCode === 'not_configured'}
		<div class="{styles.banner} {styles.bannerError}" role="alert" data-testid="ws-banner-not-configured">
			{errorMessage ??
				'Web search is not configured. Add a Google Custom Search API key and Engine ID to enable this tool.'}
			<div class={styles.bannerAction}>
				<button class={styles.bannerButton} type="button" onclick={openSettings}>
					Open settings →
				</button>
			</div>
		</div>
	{:else if isError}
		<div class="{styles.banner} {styles.bannerError}" role="alert" data-testid="ws-banner-error">
			{errorMessage ?? 'Search failed.'}
		</div>
	{:else if success}
		{#if success.count === 0 && success.filtered_by_policy}
			<div class="{styles.banner} {styles.bannerNeutral}" role="status" data-testid="ws-banner-policy">
				All provider results were dropped by this workspace's domain policy.
				Review the allow/deny lists in Settings → Tools → Web Search.
			</div>
		{:else if success.count === 0}
			<div class="{styles.banner} {styles.bannerNeutral}" role="status" data-testid="ws-banner-empty">
				No results. Try a different query.
			</div>
		{:else}
			{#if success.cached}
				<div class="{styles.banner} {styles.bannerNeutral}" role="status" data-testid="ws-banner-cached">
					Served from local cache. Results are re-filtered through the current policy on read.
				</div>
			{/if}
			{#if success.filtered_by_policy}
				<div class="{styles.banner} {styles.bannerNeutral}" role="status" data-testid="ws-banner-filtered">
					Some results were dropped by this workspace's domain policy.
				</div>
			{/if}
			<WebSearchResults results={success.results} />
		{/if}
	{/if}
</div>
