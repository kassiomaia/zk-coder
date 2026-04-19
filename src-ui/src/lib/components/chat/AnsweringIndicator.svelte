<script lang="ts">
	import styles from './AnsweringIndicator.module.css';
	import type { AgentStatus } from '$lib/utils/agentStatus';

	interface Props {
		status?: AgentStatus | null;
		active?: boolean;
	}

	let { status }: Props = $props();

	type IndicatorState = 'thinking' | 'tool' | 'stall' | 'complete';

	const indicatorState = $derived<IndicatorState>(
		(() => {
			if (!status) return 'thinking';
			if (!status.active && status.icon === 'check') return 'complete';
			if (status.stallWarning) return 'stall';
			const hint = status.icon ?? 'sparkle';
			if (hint === 'file' || hint === 'edit' || hint === 'terminal' || hint === 'scan')
				return 'tool';
			return 'thinking';
		})()
	);

	const displayVerb = $derived(
		(() => {
			if (!status) return 'thinking';
			if (!status.active && status.icon === 'check') return 'done';
			if (status.stallWarning) return 'still working';
			return status.verb ?? status.text ?? 'thinking';
		})()
	);

	const displayTarget = $derived(status?.stallWarning ? '' : (status?.target ?? ''));

	const showCheck = $derived(indicatorState === 'complete');

	function formatElapsed(seconds: number | undefined): string {
		if (!seconds || seconds < 2) return '';
		if (seconds < 60) return `${seconds}s`;
		const m = Math.floor(seconds / 60);
		const s = seconds % 60;
		return `${m}m ${s.toString().padStart(2, '0')}s`;
	}

	const elapsedLabel = $derived(formatElapsed(status?.elapsedSeconds));

	const stateClass = $derived(
		(() => {
			switch (indicatorState) {
				case 'thinking':
					return styles.stateThinking;
				case 'tool':
					return styles.stateTool;
				case 'stall':
					return styles.stateStall;
				case 'complete':
					return styles.stateComplete;
			}
		})()
	);
</script>

<div
	class={[styles.indicator, stateClass].join(' ')}
	role="status"
	aria-live="polite"
	aria-label="Agent status"
>
	<div class={styles.bars} aria-hidden="true">
		<div class={styles.bar}></div>
		<div class={styles.bar}></div>
		<div class={styles.bar}></div>
		<div class={styles.bar}></div>
		<div class={styles.bar}></div>
	</div>

	<div class={styles.statusText}>
		{#if showCheck}
			<span class={styles.check} aria-hidden="true">✓</span>
		{/if}
		<span class={styles.statusVerb}>{displayVerb}</span>
		{#if displayTarget}
			<span class={styles.statusTarget}>{displayTarget}</span>
		{/if}
	</div>

	{#if elapsedLabel}
		<span class={styles.elapsed} aria-hidden="true">{elapsedLabel}</span>
	{/if}
</div>
