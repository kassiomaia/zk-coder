<script lang="ts">
	/**
	 * WebSearchCredentials — Google Custom Search API key + CX form.
	 *
	 * Mirrors docs/tools/web_search/mockups.html panel 07 and the contract in
	 * docs/archive/tools/web_search/plan.md Appendix B.3.
	 *
	 * Security-sensitive defaults:
	 * - API key input is `type="password"` by default; a reveal toggle flips
	 *   to `type="text"` for user confirmation.
	 * - The component never renders the stored key until the user explicitly
	 *   reveals it. On mount, if a key is already stored, the field shows a
	 *   masked placeholder (`••••••`) that the user must clear before typing.
	 * - API-key state is cleared on unmount to minimize JS heap residency.
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

	const MASKED_PLACEHOLDER = '••••••••••••••••••••••';

	let apiKey = $state('');
	let cx = $state('');
	let revealed = $state(false);
	let loading = $state(true);
	let saving = $state(false);
	let deleting = $state(false);
	let testState = $state<TestState>({ kind: 'idle' });
	let hasStoredKey = $state(false);
	let apiKeyTouched = $state(false);
	let validationError = $state<string | null>(null);

	const isTesting = $derived(testState.kind === 'running');
	// The API key is ready to submit when either the user typed a new value
	// or the masked placeholder is still in place (we accept "no change").
	// We DO NOT auto-send the masked placeholder to the backend — if the user
	// never touched the input, `saveCredentials` skips the API-key update.
	const apiKeyReady = $derived(
		apiKeyTouched ? apiKey.trim().length > 0 : hasStoredKey
	);
	const canSave = $derived(
		!saving && !isTesting && apiKeyReady && cx.trim() !== ''
	);

	async function load() {
		loading = true;
		try {
			hasStoredKey = await hasToolCredentials(WEB_SEARCH_TOOL_ID);
			const stored = hasStoredKey ? await getToolCredentials(WEB_SEARCH_TOOL_ID) : null;
			if (stored) {
				apiKey = MASKED_PLACEHOLDER;
				cx = stored.cx;
			} else {
				apiKey = '';
				cx = '';
			}
		} catch (err) {
			validationError = err instanceof Error ? err.message : 'Failed to load credentials.';
		} finally {
			loading = false;
			apiKeyTouched = false;
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

	function handleApiKeyInput(ev: Event) {
		apiKey = (ev.target as HTMLInputElement).value;
		apiKeyTouched = true;
		testState = { kind: 'idle' };
		validationError = null;
	}

	function handleCxInput(ev: Event) {
		cx = (ev.target as HTMLInputElement).value;
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
		try {
			// If the user never touched the API key, we must not overwrite the
			// stored value with the masked placeholder. Require an explicit
			// edit to change the key.
			if (!apiKeyTouched) {
				// Still allow a CX-only update by re-sending the stored API key
				// indirectly: we fetch once and forward. This avoids round-trip
				// flicker and keeps the key in memory only for this call.
				const stored = await getToolCredentials(WEB_SEARCH_TOOL_ID);
				if (!stored) {
					validationError =
						'No stored credentials found. Enter an API key to save.';
					return;
				}
				await setToolCredentials(WEB_SEARCH_TOOL_ID, {
					apiKey: stored.apiKey,
					cx: cx.trim()
				});
			} else {
				await setToolCredentials(WEB_SEARCH_TOOL_ID, {
					apiKey: apiKey.trim(),
					cx: cx.trim()
				});
			}
			testState = { kind: 'idle' };
			hasStoredKey = true;
			apiKeyTouched = false;
			// Reset to masked view on successful save.
			if (apiKey !== MASKED_PLACEHOLDER) {
				apiKey = MASKED_PLACEHOLDER;
				revealed = false;
			}
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
			apiKeyTouched = false;
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
				Required. Stored in <code>coder_config.db</code>. Never transmitted beyond Google.
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
				value={apiKey}
				oninput={handleApiKeyInput}
				placeholder={hasStoredKey ? MASKED_PLACEHOLDER : 'AIzaSy…'}
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
			value={cx}
			oninput={handleCxInput}
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
