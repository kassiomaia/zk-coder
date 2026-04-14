<script lang="ts">
	/**
	 * WebSearchPolicy — per-workspace allow/deny domain editor.
	 *
	 * Mirrors docs/tools/web_search/mockups.html panel 08 and the archived
	 * plan (docs/archive/tools/web_search/plan.md Appendix B.4). Policy is
	 * scoped by canonicalized workspace path; when no workspace is active,
	 * the editor renders in a disabled/empty state.
	 */
	import Button from '../../shared/Button.svelte';
	import {
		getWorkspaceSearchPolicy,
		setWorkspaceSearchPolicy
	} from '$lib/api/workspacePolicy';
	import { findListConflicts, validateDomain } from './domain';
	import styles from './WebSearchPolicy.module.css';

	interface Props {
		/** Canonicalized workspace path; '' means no workspace active. */
		workspacePath: string;
		/** Tilde-compressed display path for the header line. */
		workspaceDisplayPath?: string;
		onSaved?: () => void;
	}

	let { workspacePath, workspaceDisplayPath, onSaved }: Props = $props();

	let allowlist = $state<string[]>([]);
	let denylist = $state<string[]>([]);
	let allowDraft = $state('');
	let denyDraft = $state('');
	let allowError = $state<string | null>(null);
	let denyError = $state<string | null>(null);
	let loading = $state(false);
	let saving = $state(false);
	let snapshot = $state<string>('[],[]');
	let submitError = $state<string | null>(null);

	const enabled = $derived(workspacePath !== '');
	const conflicts = $derived(findListConflicts(allowlist, denylist));
	const dirty = $derived(
		JSON.stringify([allowlist, denylist]) !== snapshot
	);

	$effect(() => {
		void workspacePath;
		loadPolicy();
	});

	async function loadPolicy() {
		if (!enabled) {
			allowlist = [];
			denylist = [];
			snapshot = JSON.stringify([[], []]);
			return;
		}
		loading = true;
		try {
			const stored = await getWorkspaceSearchPolicy(workspacePath);
			allowlist = [...stored.allowlist];
			denylist = [...stored.denylist];
			snapshot = JSON.stringify([allowlist, denylist]);
			allowDraft = '';
			denyDraft = '';
			allowError = null;
			denyError = null;
			submitError = null;
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	function addToList(list: 'allow' | 'deny') {
		const draft = list === 'allow' ? allowDraft : denyDraft;
		const result = validateDomain(draft);
		if (!result.valid) {
			if (list === 'allow') allowError = result.reason ?? 'Invalid domain.';
			else denyError = result.reason ?? 'Invalid domain.';
			return;
		}
		const normalized = result.normalized!;
		if (list === 'allow') {
			if (!allowlist.includes(normalized)) {
				allowlist = [...allowlist, normalized];
			}
			allowDraft = '';
			allowError = null;
		} else {
			if (!denylist.includes(normalized)) {
				denylist = [...denylist, normalized];
			}
			denyDraft = '';
			denyError = null;
		}
	}

	function removeFrom(list: 'allow' | 'deny', domain: string) {
		if (list === 'allow') {
			allowlist = allowlist.filter((d) => d !== domain);
		} else {
			denylist = denylist.filter((d) => d !== domain);
		}
	}

	function onKeyDown(ev: KeyboardEvent, list: 'allow' | 'deny') {
		if (ev.key === 'Enter') {
			ev.preventDefault();
			addToList(list);
		}
	}

	async function save() {
		if (!enabled || !dirty || saving) return;
		saving = true;
		submitError = null;
		try {
			await setWorkspaceSearchPolicy(workspacePath, allowlist, denylist);
			snapshot = JSON.stringify([allowlist, denylist]);
			onSaved?.();
		} catch (err) {
			submitError = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function reset() {
		loadPolicy();
	}
</script>

<section class="{styles.card} {!enabled ? styles.disabled : ''}" aria-labelledby="ws-policy-title">
	<header class={styles.head}>
		<div>
			<h3 id="ws-policy-title" class={styles.title}>Domain policy</h3>
			<p class={styles.hint}>
				{#if enabled}
					Active workspace:
					<code>{workspaceDisplayPath ?? workspacePath}</code>
				{:else}
					Global · no workspace active
				{/if}
			</p>
		</div>
	</header>

	{#if !enabled}
		<div class={styles.empty}>
			Domain policy is scoped per workspace. Open an agent session with a workspace to edit its
			allow/deny lists.
		</div>
	{:else}
		<div class={styles.row}>
			<span class={styles.label}>
				Allowlist
				<span class={styles.labelAux}>· if set, only these domains may appear</span>
			</span>
			<div class={styles.chips} role="list" aria-label="Allowlist">
				{#each allowlist as domain (domain)}
					<span class="{styles.chip} {styles.chipAllow}" role="listitem">
						{domain}
						<button
							class={styles.chipRemove}
							type="button"
							onclick={() => removeFrom('allow', domain)}
							aria-label="Remove {domain} from allowlist"
						>×</button>
					</span>
				{/each}
				<input
					class={styles.chipInput}
					type="text"
					placeholder="add domain…"
					aria-label="Add domain to allowlist"
					autocomplete="off"
					spellcheck="false"
					bind:value={allowDraft}
					onkeydown={(ev) => onKeyDown(ev, 'allow')}
					data-testid="allow-input"
				/>
			</div>
			{#if allowError}
				<div class={styles.inlineError} role="alert">{allowError}</div>
			{/if}
		</div>

		<div class={styles.row}>
			<span class={styles.label}>
				Denylist
				<span class={styles.labelAux}>· always blocked, overrides allowlist</span>
			</span>
			<div class={styles.chips} role="list" aria-label="Denylist">
				{#each denylist as domain (domain)}
					<span class="{styles.chip} {styles.chipDeny}" role="listitem">
						{domain}
						<button
							class={styles.chipRemove}
							type="button"
							onclick={() => removeFrom('deny', domain)}
							aria-label="Remove {domain} from denylist"
						>×</button>
					</span>
				{/each}
				<input
					class={styles.chipInput}
					type="text"
					placeholder="add domain…"
					aria-label="Add domain to denylist"
					autocomplete="off"
					spellcheck="false"
					bind:value={denyDraft}
					onkeydown={(ev) => onKeyDown(ev, 'deny')}
					data-testid="deny-input"
				/>
			</div>
			{#if denyError}
				<div class={styles.inlineError} role="alert">{denyError}</div>
			{/if}
		</div>

		{#if conflicts.length > 0}
			<div class={styles.warning} role="status" data-testid="conflict-warning">
				<strong>{conflicts.length}</strong>
				{conflicts.length === 1 ? 'entry appears' : 'entries appear'} in both lists
				(<span class={styles.warningTerm}>{conflicts.join(', ')}</span>).
				The denylist wins at enforcement time.
			</div>
		{/if}

		{#if submitError}
			<div class={styles.inlineError} role="alert">{submitError}</div>
		{/if}

		<div class={styles.actions}>
			<Button
				variant="primary"
				size="sm"
				disabled={!dirty || saving || loading}
				loading={saving}
				onclick={save}
			>
				Save policy
			</Button>
			<div class={styles.spacer}></div>
			<Button variant="ghost" size="sm" disabled={!dirty || saving || loading} onclick={reset}>
				Reset
			</Button>
		</div>
	{/if}
</section>
