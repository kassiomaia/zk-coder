<script lang="ts">
	/**
	 * IntentIndicator Component
	 * Badge showing the detected intent from user input.
	 * Supports both frontend-only mode (simple patterns) and backend NLP mode.
	 */
	import styles from './IntentIndicator.module.css';
	import type { Intent, IntentClassification } from '$lib/types/nlp';
	import { getIntentLabel, INTENT_LABELS } from '$lib/types/nlp';

	interface Props {
		/** The detected intent (can be Intent type or simple string for frontend mode) */
		intent: Intent | string;
		/** Confidence level (0-1) */
		confidence?: number;
		/** Show confidence percentage */
		showConfidence?: boolean;
		/** Show loading state */
		loading?: boolean;
		/** Full classification result (optional, for backend mode) */
		classification?: IntentClassification | null;
		/** Error message; when set and not loading, the badge renders the error state */
		error?: string | null;
	}

	let {
		intent,
		confidence = 0,
		showConfidence = false,
		loading = false,
		classification,
		error = null
	}: Props = $props();

	let hasError = $derived(!loading && !!error);

	// Map legacy/simple intents to display icons
	const intentIcons: Record<string, string> = {
		// Backend intents (snake_case)
		generate_code: 'code',
		generate_function: 'code',
		generate_class: 'code',
		generate_test: 'check',
		refactor_code: 'refresh',
		optimize_code: 'zap',
		explain_code: 'book',
		analyze_code: 'search',
		find_bugs: 'bug',
		document_code: 'file-text',
		find_file: 'file',
		find_symbol: 'at-sign',
		goto_definition: 'arrow-right',
		find_references: 'git-branch',
		run_tests: 'check',
		build_project: 'box',
		format_code: 'align-left',
		lint_code: 'alert-circle',
		git_status: 'git-branch',
		git_commit: 'git-commit',
		git_diff: 'git-merge',
		git_log: 'list',
		// Legacy frontend intents
		generate: 'code',
		explain: 'book',
		fix: 'wrench',
		refactor: 'refresh',
		test: 'check',
		review: 'eye',
		document: 'file-text'
	};

	// Get display label - use NLP labels if available
	let displayLabel = $derived(() => {
		// Check if it's a known Intent type
		if (intent in INTENT_LABELS) {
			return getIntentLabel(intent as Intent);
		}
		// Capitalize first letter for unknown intents
		return intent.charAt(0).toUpperCase() + intent.slice(1).replace(/_/g, ' ');
	});

	let iconName = $derived(intentIcons[intent] || 'terminal');
	let confidencePercent = $derived(confidence > 0 ? Math.round(confidence * 100) : null);
	let confidenceLevel = $derived(() => {
		if (confidence > 0.8) return 'high';
		if (confidence > 0.5) return 'medium';
		return 'low';
	});
</script>

<div
	class={[
		styles.badge,
		loading && styles.loading,
		hasError && styles.error,
		!hasError && confidenceLevel() && styles[`confidence-${confidenceLevel()}`]
	]
		.filter(Boolean)
		.join(' ')}
	role="status"
	aria-live="polite"
	aria-label={hasError ? 'Intent detection unavailable' : `Detected intent: ${displayLabel()}`}
	title={hasError ? (error ?? undefined) : undefined}
>
	{#if loading}
		<span class={styles.spinner}>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20">
					<animateTransform
						attributeName="transform"
						type="rotate"
						from="0 12 12"
						to="360 12 12"
						dur="1s"
						repeatCount="indefinite"
					/>
				</circle>
			</svg>
		</span>
	{:else if hasError}
		<span class={styles.icon}>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<circle cx="12" cy="12" r="10" />
				<line x1="12" y1="8" x2="12" y2="12" />
				<line x1="12" y1="16" x2="12.01" y2="16" />
			</svg>
		</span>
	{:else}
		<span class={styles.icon}>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				{#if iconName === 'code'}
					<polyline points="16 18 22 12 16 6" />
					<polyline points="8 6 2 12 8 18" />
				{:else if iconName === 'book'}
					<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
					<path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
				{:else if iconName === 'wrench'}
					<path
						d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
					/>
				{:else if iconName === 'refresh'}
					<polyline points="23 4 23 10 17 10" />
					<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
				{:else if iconName === 'check'}
					<polyline points="20 6 9 17 4 12" />
				{:else if iconName === 'eye'}
					<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
					<circle cx="12" cy="12" r="3" />
				{:else if iconName === 'file-text'}
					<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
					<polyline points="14 2 14 8 20 8" />
					<line x1="16" y1="13" x2="8" y2="13" />
					<line x1="16" y1="17" x2="8" y2="17" />
				{:else if iconName === 'zap'}
					<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
				{:else if iconName === 'search'}
					<circle cx="11" cy="11" r="8" />
					<line x1="21" y1="21" x2="16.65" y2="16.65" />
				{:else if iconName === 'bug'}
					<path d="m8 2 1.88 1.88M14.12 3.88 16 2M9 7.13v-1a3.003 3.003 0 1 1 6 0v1.003" />
					<path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6" />
					<path
						d="M12 20v-9M6.53 9C4.6 8.8 3 7.1 3 5M6 13H2M6 17H3M17.47 9C19.4 8.8 21 7.1 21 5M18 13h4M18 17h4"
					/>
				{:else if iconName === 'file'}
					<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
					<polyline points="14 2 14 8 20 8" />
				{:else if iconName === 'at-sign'}
					<circle cx="12" cy="12" r="4" />
					<path d="M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-3.92 7.94" />
				{:else if iconName === 'arrow-right'}
					<line x1="5" y1="12" x2="19" y2="12" />
					<polyline points="12 5 19 12 12 19" />
				{:else if iconName === 'git-branch'}
					<line x1="6" y1="3" x2="6" y2="15" />
					<circle cx="18" cy="6" r="3" />
					<circle cx="6" cy="18" r="3" />
					<path d="M18 9a9 9 0 0 1-9 9" />
				{:else if iconName === 'box'}
					<path
						d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"
					/>
					<polyline points="3.27 6.96 12 12.01 20.73 6.96" />
					<line x1="12" y1="22.08" x2="12" y2="12" />
				{:else if iconName === 'align-left'}
					<line x1="17" y1="10" x2="3" y2="10" />
					<line x1="21" y1="6" x2="3" y2="6" />
					<line x1="21" y1="14" x2="3" y2="14" />
					<line x1="17" y1="18" x2="3" y2="18" />
				{:else if iconName === 'alert-circle'}
					<circle cx="12" cy="12" r="10" />
					<line x1="12" y1="8" x2="12" y2="12" />
					<line x1="12" y1="16" x2="12.01" y2="16" />
				{:else if iconName === 'git-commit'}
					<circle cx="12" cy="12" r="4" />
					<line x1="1.05" y1="12" x2="7" y2="12" />
					<line x1="17.01" y1="12" x2="22.96" y2="12" />
				{:else if iconName === 'git-merge'}
					<circle cx="18" cy="18" r="3" />
					<circle cx="6" cy="6" r="3" />
					<path d="M6 21V9a9 9 0 0 0 9 9" />
				{:else if iconName === 'list'}
					<line x1="8" y1="6" x2="21" y2="6" />
					<line x1="8" y1="12" x2="21" y2="12" />
					<line x1="8" y1="18" x2="21" y2="18" />
					<line x1="3" y1="6" x2="3.01" y2="6" />
					<line x1="3" y1="12" x2="3.01" y2="12" />
					<line x1="3" y1="18" x2="3.01" y2="18" />
				{:else}
					<polyline points="4 17 10 11 4 5" />
					<line x1="12" y1="19" x2="20" y2="19" />
				{/if}
			</svg>
		</span>
	{/if}
	<span class={styles.label}>{hasError ? 'Intent unavailable' : displayLabel()}</span>
	{#if showConfidence && confidencePercent !== null && !hasError}
		<span class={styles.confidence}>{confidencePercent}%</span>
	{/if}
</div>
