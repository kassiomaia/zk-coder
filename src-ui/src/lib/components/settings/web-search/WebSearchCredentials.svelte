<script lang="ts">
	/**
	 * WebSearchCredentials — Google Custom Search API key + CX form.
	 *
	 * Mirrors docs/tools/web_search/mockups.html panel 07 and the contract in
	 * docs/archive/tools/web_search/plan.md Appendix B.3.
	 *
	 * Security / UX discipline:
	 * - API key input is `type="password"` by default; a reveal toggle flips
	 *   to `type="text"` so the user can verify what they typed.
	 * - The real stored key is NEVER stuffed back into the input's `value`.
	 *   On load, if a key is already stored, the input stays empty and the
	 *   HTML `placeholder` attribute communicates "existing key stored —
	 *   leave blank to keep". Save with an empty API-key input re-uses the
	 *   stored value (CX-only updates); Save with a non-empty input replaces
	 *   the stored key.
	 * - On Clear, both fields are wiped and the stored row is deleted.
	 */
	import Button from '../../shared/Button.svelte';
	import { CheckIcon, AlertCircleIcon } from '$lib/icons';
	import {
		deleteToolCredentials,
		getToolCredentials,
		hasToolCredentials,
		setToolCredentials,
		testWebSearchCredentials,
		WEB_SEARCH_TOOL_ID
	} from '$lib/api/toolCredentials';
	import styles from './WebSearchCredentials.module.css';

	interface Props {
		/** Fires after a successful Save/Clear so the parent can refresh stats. */
		onSaved?: () => void;
	}

	let { onSaved }: Props = $props();

	type TestState =
		| { kind: 'idle' }
		| { kind: 'running' }
		| { kind: 'success'; message: string; at: Date }
		| { kind: 'failure'; message: string; at: Date };

	let apiKey = $state('');
	let cx = $state('');
	let revealed = $state(false);
	let loading = $state(true);
	let saving = $state(false);
	let deleting = $state(false);
	let testState = $state<TestState>({ kind: 'idle' });
	let hasStoredKey = $state(false);
	let validationError = $state<string | null>(null);

	const isTesting = $derived(testState.kind === 'running');

	// Save is enabled when CX is non-empty AND we either have a newly-typed
	// API key or an existing stored key to keep. Typing into the API-key
	// input is treated as an explicit replacement; leaving it empty means
	// "CX-only update — don't touch the stored key".
	const canSave = $derived(
		!saving &&
			!isTesting &&
			cx.trim() !== '' &&
			(apiKey.trim() !== '' || hasStoredKey)
	);

	const apiKeyPlaceholder = $derived(
		hasStoredKey ? 'Leave blank to keep the stored key' : 'AIzaSy…'
	);

	async function load() {
		loading = true;
		try {
			hasStoredKey = await hasToolCredentials(WEB_SEARCH_TOOL_ID);
			const stored = hasStoredKey
				? await getToolCredentials(WEB_SEARCH_TOOL_ID)
				: null;
			// Never prefill the API key — users shouldn't see the real value
			// unless they explicitly click Reveal after saving a new one.
			apiKey = '';
			cx = stored?.cx ?? '';
		} catch (err) {
			validationError =
				err instanceof Error ? err.message : 'Failed to load credentials.';
		} finally {
			loading = false;
			revealed = false;
		}
	}

	$effect(() => {
		load();
		return () => {
			apiKey = '';
			revealed = false;
		};
	});

	function clearFeedback() {
		testState = { kind: 'idle' };
		validationError = null;
	}

	function toggleReveal() {
		revealed = !revealed;
	}

	async function saveCredentials() {
		if (!canSave) return;
		saving = true;
		validationError = null;
		const trimmedApiKey = apiKey.trim();
		const trimmedCx = cx.trim();
		try {
			if (trimmedApiKey === '') {
				// CX-only update: re-fetch the stored key and forward with
				// the new CX. This keeps the key secret end-to-end except for
				// the one call that persists the updated row.
				const stored = await getToolCredentials(WEB_SEARCH_TOOL_ID);
				if (!stored) {
					validationError =
						'No stored credentials found. Enter an API key to save.';
					return;
				}
				await setToolCredentials(WEB_SEARCH_TOOL_ID, {
					apiKey: stored.apiKey,
					cx: trimmedCx
				});
			} else {
				await setToolCredentials(WEB_SEARCH_TOOL_ID, {
					apiKey: trimmedApiKey,
					cx: trimmedCx
				});
			}
			testState = { kind: 'idle' };
			hasStoredKey = true;
			// Wipe the input value so the next edit starts from empty and
			// the placeholder communicates "stored key in place".
			apiKey = '';
			revealed = false;
			onSaved?.();
		} catch (err) {
			validationError = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	async function runTest() {
		if (saving || isTesting) return;
		testState = { kind: 'running' };
		try {
			const result = await testWebSearchCredentials();
			testState = result.valid
				? { kind: 'success', message: result.message, at: new Date() }
				: { kind: 'failure', message: result.message, at: new Date() };
		} catch (err) {
			testState = {
				kind: 'failure',
				message: err instanceof Error ? err.message : String(err),
				at: new Date()
			};
		}
	}

	async function clearCredentials() {
		deleting = true;
		validationError = null;
		try {
			await deleteToolCredentials(WEB_SEARCH_TOOL_ID);
			apiKey = '';
			cx = '';
			hasStoredKey = false;
			testState = { kind: 'idle' };
			onSaved?.();
		} catch (err) {
			validationError = err instanceof Error ? err.message : String(err);
		} finally {
			deleting = false;
		}
	}

	function relativeTime(date: Date): string {
		const diffMs = Date.now() - date.getTime();
		const mins = Math.floor(diffMs / 60_000);
		if (mins < 1) return 'just now';
		if (mins === 1) return '1 min ago';
		if (mins < 60) return `${mins} min ago`;
		const hrs = Math.floor(mins / 60);
		if (hrs === 1) return '1 h ago';
		return `${hrs} h ago`;
	}
</script>

<section class={styles.card} aria-labelledby="web-search-creds-title">
	<header class={styles.head}>
		<div>
			<h3 id="web-search-creds-title" class={styles.title}>Google Custom Search</h3>
			<p class={styles.hint}>
				Stored securely on this device. Only sent to Google when the assistant runs a search.
			</p>
		</div>
		{#if testState.kind === 'success'}
			<span
				class="{styles.status} {styles.statusVerified}"
				role="status"
				aria-live="polite"
				data-testid="verified-badge"
			>
				<CheckIcon size={10} />
				Verified {relativeTime(testState.at)}
			</span>
		{:else if testState.kind === 'failure'}
			<span
				class="{styles.status} {styles.statusFailed}"
				role="status"
				aria-live="polite"
				data-testid="failed-badge"
			>
				<AlertCircleIcon size={10} />
				Test failed
			</span>
		{/if}
	</header>

	<div class={styles.row}>
		<label for="web-search-api-key" class={styles.label}>API key</label>
		<div class={styles.inputWrap}>
			<input
				id="web-search-api-key"
				class={styles.input}
				type={revealed ? 'text' : 'password'}
				autocomplete="off"
				spellcheck="false"
				disabled={loading}
				bind:value={apiKey}
				oninput={clearFeedback}
				placeholder={apiKeyPlaceholder}
				aria-describedby="web-search-api-key-hint"
				data-testid="api-key-input"
			/>
			<button
				class={styles.reveal}
				type="button"
				onclick={toggleReveal}
				aria-label={revealed ? 'Hide API key' : 'Show API key'}
				aria-pressed={revealed}
				disabled={loading}
				data-testid="reveal-toggle"
			>
				{revealed ? 'Hide' : 'Show'}
			</button>
		</div>
	</div>

	<div class={styles.row}>
		<label for="web-search-cx" class={styles.label}>
			Search Engine ID
			<span class={styles.labelAux}>(cx)</span>
		</label>
		<input
			id="web-search-cx"
			class="{styles.input} {styles.inputPlain}"
			type="text"
			autocomplete="off"
			spellcheck="false"
			disabled={loading}
			bind:value={cx}
			oninput={clearFeedback}
			placeholder="017576662512468239146:omuauf_lfve"
			data-testid="cx-input"
		/>
	</div>

	<div class={styles.actions}>
		<Button
			variant="primary"
			size="sm"
			disabled={!canSave}
			loading={saving}
			onclick={saveCredentials}
		>
			Save
		</Button>
		<Button
			variant="secondary"
			size="sm"
			disabled={isTesting || saving || !hasStoredKey}
			loading={isTesting}
			onclick={runTest}
		>
			Test
		</Button>
		<div class={styles.spacer}></div>
		<Button
			variant="ghost"
			size="sm"
			disabled={deleting || saving || !hasStoredKey}
			onclick={clearCredentials}
		>
			Clear
		</Button>
	</div>

	{#if validationError}
		<div class="{styles.feedback} {styles.feedbackError}" role="alert" data-testid="error-banner">
			{validationError}
		</div>
	{:else if testState.kind === 'failure'}
		<div class="{styles.feedback} {styles.feedbackError}" role="alert" data-testid="test-failure">
			{testState.message}
		</div>
	{:else if testState.kind === 'success'}
		<div class="{styles.feedback} {styles.feedbackSuccess}" role="status" data-testid="test-success">
			{testState.message}
		</div>
	{/if}
</section>
