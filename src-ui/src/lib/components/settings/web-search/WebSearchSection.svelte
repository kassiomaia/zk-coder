<script lang="ts">
	/**
	 * WebSearchSection — composition root for the Settings > Tools > Web Search area.
	 *
	 * Orchestrates credentials, per-workspace policy, and cache controls. All
	 * three sub-components fetch their own state from the backend, so this
	 * container only provides layout and a shared refresh token so a change
	 * in one panel can nudge the others (e.g. saving credentials refreshes
	 * cache stats so a freshly-enabled Test button reads the latest counts).
	 */
	import WebSearchCredentials from './WebSearchCredentials.svelte';
	import WebSearchPolicy from './WebSearchPolicy.svelte';
	import WebSearchCacheControls from './WebSearchCacheControls.svelte';
	import styles from './WebSearchSection.module.css';

	interface Props {
		/** Canonicalized workspace path; '' means no workspace active. */
		workspacePath?: string;
		/** Tilde-compressed display path for policy header. */
		workspaceDisplayPath?: string;
	}

	let { workspacePath = '', workspaceDisplayPath }: Props = $props();

	let refreshToken = $state(0);

	function bumpRefresh() {
		refreshToken = refreshToken + 1;
	}
</script>

<div class={styles.section}>
	<p class={styles.preface}>
		Production Google Custom Search integration for the <code>search_web_search</code> tool.
		Gated by the <code>CODER_LLM_TOOL_WEB_SEARCH</code> feature flag — the tool is hidden from the
		LLM until a key is saved here and the flag is on.
	</p>

	<WebSearchCredentials onSaved={bumpRefresh} />
	<WebSearchPolicy
		{workspacePath}
		{workspaceDisplayPath}
		onSaved={bumpRefresh}
	/>
	<WebSearchCacheControls {workspacePath} {refreshToken} />
</div>
