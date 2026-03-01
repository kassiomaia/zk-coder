<script lang="ts">
	/**
	 * IntentIndicator Component
	 * Badge showing the detected intent from user input.
	 */
	import styles from './IntentIndicator.module.css';

	interface Props {
		/** The detected intent type */
		intent: string;
		/** Confidence level (0-1) */
		confidence?: number;
		/** Show confidence percentage */
		showConfidence?: boolean;
	}

	let { intent, confidence, showConfidence = false }: Props = $props();

	// Map intents to display labels and icons
	const intentConfig: Record<string, { label: string; icon: string }> = {
		generate: { label: 'Generate', icon: 'code' },
		explain: { label: 'Explain', icon: 'book' },
		fix: { label: 'Fix', icon: 'wrench' },
		refactor: { label: 'Refactor', icon: 'refresh' },
		test: { label: 'Test', icon: 'check' },
		review: { label: 'Review', icon: 'eye' },
		document: { label: 'Document', icon: 'file-text' }
	};

	let config = $derived(intentConfig[intent] || { label: intent, icon: 'terminal' });
	let confidencePercent = $derived(confidence ? Math.round(confidence * 100) : null);
</script>

<div class={styles.badge} role="status" aria-label="Detected intent: {config.label}">
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
			{#if config.icon === 'code'}
				<polyline points="16 18 22 12 16 6" />
				<polyline points="8 6 2 12 8 18" />
			{:else if config.icon === 'book'}
				<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
				<path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
			{:else if config.icon === 'wrench'}
				<path
					d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"
				/>
			{:else if config.icon === 'refresh'}
				<polyline points="23 4 23 10 17 10" />
				<path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
			{:else if config.icon === 'check'}
				<polyline points="20 6 9 17 4 12" />
			{:else if config.icon === 'eye'}
				<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
				<circle cx="12" cy="12" r="3" />
			{:else if config.icon === 'file-text'}
				<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
				<polyline points="14 2 14 8 20 8" />
				<line x1="16" y1="13" x2="8" y2="13" />
				<line x1="16" y1="17" x2="8" y2="17" />
			{:else}
				<polyline points="4 17 10 11 4 5" />
				<line x1="12" y1="19" x2="20" y2="19" />
			{/if}
		</svg>
	</span>
	<span class={styles.label}>{config.label}</span>
	{#if showConfidence && confidencePercent !== null}
		<span class={styles.confidence}>{confidencePercent}%</span>
	{/if}
</div>
