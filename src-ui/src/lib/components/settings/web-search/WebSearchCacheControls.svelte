<script lang="ts">
	/**
	 * WebSearchCacheControls — entry-count display + clear actions.
	 *
	 * Mirrors docs/tools/web_search/mockups.html panel 09 and the archived
	 * plan (docs/archive/tools/web_search/plan.md Appendix B.5). Per-workspace
	 * clear is only offered when a workspace is active; the "Clear all"
	 * action is always available.
	 */
	import Button from '../../shared/Button.svelte';
	import {
		clearWebSearchCache,
		getWebSearchCacheStats,
		type WebSearchCacheStats
	} from '$lib/api/webSearchCache';
	import { formatTtl } from './domain';
	import styles from './WebSearchCacheControls.module.css';

	interface Props {
		/** Canonicalized workspace path; '' means no workspace active. */
		workspacePath: string;
		/** Trigger used by the parent to force a refresh after external changes. */
		refreshToken?: number;
	}

	let { workspacePath, refreshToken }: Props = $props();

	type Feedback =
		| { kind: 'none' }
		| { kind: 'info'; message: string }
		| { kind: 'error'; message: string };

	let stats = $state<WebSearchCacheStats | null>(null);
	let loading = $state(false);
	let clearing = $state(false);
	let feedback = $state<Feedback>({ kind: 'none' });

	const hasWorkspace = $derived(workspacePath !== '');

	$effect(() => {
		void refreshToken;
		loadStats();
	});

	async function loadStats() {
		loading = true;
		try {
			stats = await getWebSearchCacheStats();
		} catch (err) {
			feedback = {
				kind: 'error',
				message: err instanceof Error ? err.message : String(err)
			};
		} finally {
			loading = false;
		}
	}

	async function clearScoped(scope: 'workspace' | 'all') {
		if (clearing) return;
		clearing = true;
		feedback = { kind: 'none' };
		try {
			const removed = await clearWebSearchCache(
				scope === 'workspace' ? workspacePath : undefined
			);
			feedback = {
				kind: 'info',
				message:
					removed === 0
						? 'Cache was already empty.'
						: `Removed ${removed} cache ${removed === 1 ? 'entry' : 'entries'}.`
			};
			await loadStats();
		} catch (err) {
			feedback = {
				kind: 'error',
				message: err instanceof Error ? err.message : String(err)
			};
		} finally {
			clearing = false;
		}
	}
</script>

<section class={styles.card} aria-labelledby="ws-cache-title">
	<header class={styles.head}>
		<div>
			<h3 id="ws-cache-title" class={styles.title}>Cache</h3>
			<p class={styles.hint}>
				Search results are kept per workspace for 24 hours to keep things fast. Changes to the
				domain policy take effect on the next search — no need to clear the cache.
			</p>
		</div>
	</header>

	<div class={styles.grid}>
		<div class={styles.stat}>
			<div class={styles.statLabel}>Entries</div>
			<div class={styles.statValue} aria-label="{stats?.entries ?? 0} cached entries">
				{loading ? '…' : (stats?.entries ?? 0)}
			</div>
		</div>
		<div class={styles.stat}>
			<div class={styles.statLabel}>Default TTL</div>
			<div
				class={styles.statValue}
				aria-label="Default time-to-live {formatTtl(stats?.defaultTtlSecs ?? 86_400)}"
			>
				{loading ? '…' : formatTtl(stats?.defaultTtlSecs ?? 86_400)}
			</div>
		</div>
	</div>

	<div class={styles.actions}>
		<Button
			variant="ghost"
			size="sm"
			disabled={!hasWorkspace || clearing || loading}
			loading={clearing && hasWorkspace}
			onclick={() => clearScoped('workspace')}
		>
			Clear this workspace
		</Button>
		<div class={styles.spacer}></div>
		<Button
			variant="danger"
			size="sm"
			disabled={clearing || loading || (stats?.entries ?? 0) === 0}
			loading={clearing && !hasWorkspace}
			onclick={() => clearScoped('all')}
		>
			Clear all
		</Button>
	</div>

	{#if feedback.kind === 'info'}
		<div class="{styles.feedback} {styles.feedbackInfo}" role="status" data-testid="cache-info">
			{feedback.message}
		</div>
	{:else if feedback.kind === 'error'}
		<div class="{styles.feedback} {styles.feedbackError}" role="alert" data-testid="cache-error">
			{feedback.message}
		</div>
	{/if}
</section>
