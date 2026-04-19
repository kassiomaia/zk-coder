<script lang="ts">
	/**
	 * AnsweringIndicator Component
	 *
	 * A tight telemetry strip: phase dot (state-coloured, breathing) + phrase
	 * label (with letter scramble on transitions) + optional elapsed counter,
	 * wrapped in an auto-width pill with a bottom sweep rail that reads as
	 * "active work flowing through". Replaces the previous icon-per-state
	 * pattern with a single dot whose colour encodes state.
	 */
	import { onMount } from 'svelte';
	import styles from './AnsweringIndicator.module.css';
	import type { AgentStatus } from '$lib/utils/agentStatus';

	interface Props {
		status?: AgentStatus | null;
		active?: boolean;
	}

	let { status, active = true }: Props = $props();

	const SCRAMBLE_DURATION = 650;
	const SCRAMBLE_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_';
	const SCRAMBLE_LOOP_INTERVAL = 3400;
	const ROTATION_INTERVAL = 2500;

	type DotTone = 'primary' | 'warning' | 'success';

	let displayText = $state('Starting...');
	let isStable = $state(true);
	let isComplete = $state(false);

	/** In-flight RAF id. 0 means no scramble running. Kept so a new
	 *  scramble can cancel its predecessor instead of stacking; before,
	 *  bursts of `StatusPhrase` events produced overlapping RAF loops
	 *  that fought for frame budget during agent runs. */
	let animationFrame = 0;
	let mounted = false;
	let currentPhraseIndex = 0;

	/** Honour the platform reduced-motion preference. When true the
	 *  scramble is skipped entirely and the label snaps to the target. */
	const prefersReducedMotion =
		typeof window !== 'undefined' && typeof window.matchMedia === 'function'
			? window.matchMedia('(prefers-reduced-motion: reduce)').matches
			: false;

	/** Map the existing icon-type hint to a dot colour without rendering icons. */
	let dotTone = $derived<DotTone>(
		(() => {
			const hint = status?.icon ?? 'sparkle';
			if (status?.stallWarning) return 'warning';
			if (hint === 'wait') return 'warning';
			if (hint === 'check') return 'success';
			return 'primary';
		})()
	);

	let elapsedLabel = $derived(formatElapsed(status?.elapsedSeconds));

	function formatElapsed(seconds: number | undefined): string {
		if (!seconds || seconds < 2) return '';
		if (seconds < 60) return `${seconds}s`;
		const m = Math.floor(seconds / 60);
		const s = seconds % 60;
		return `${m}:${s.toString().padStart(2, '0')}`;
	}

	function easeOutQuart(x: number): number {
		return 1 - Math.pow(1 - x, 4);
	}

	function scrambleText(target: string) {
		if (displayText === target) return;

		// Cancel any in-flight scramble so a fast burst of status
		// updates doesn't stack RAF loops that all target different text.
		if (animationFrame !== 0) {
			cancelAnimationFrame(animationFrame);
			animationFrame = 0;
		}

		if (prefersReducedMotion) {
			displayText = target;
			isStable = true;
			return;
		}

		isStable = false;
		let startTime: number;
		const startText = displayText;
		const targetLength = target.length;
		const startLength = startText.length;
		const maxLength = Math.max(startLength, targetLength);

		const run = (now: number) => {
			if (!mounted) {
				animationFrame = 0;
				return;
			}
			if (startTime === undefined) startTime = now;

			const elapsed = now - startTime;
			const progress = Math.min(elapsed / SCRAMBLE_DURATION, 1);
			const easedProgress = easeOutQuart(progress);

			const resolvedCount = Math.floor(easedProgress * targetLength);

			let current = '';
			for (let i = 0; i < maxLength; i++) {
				if (i < resolvedCount) {
					current += target[i];
				} else if (i < targetLength) {
					const seed = i + Math.floor(elapsed / 50);
					current += SCRAMBLE_CHARS[seed % SCRAMBLE_CHARS.length];
				} else if (i < startLength) {
					const seed = i + Math.floor(elapsed / 50);
					current += SCRAMBLE_CHARS[seed % SCRAMBLE_CHARS.length];
				}
			}

			displayText = current;

			if (progress < 1) {
				animationFrame = requestAnimationFrame(run);
			} else {
				displayText = target;
				isStable = true;
				animationFrame = 0;
			}
		};

		animationFrame = requestAnimationFrame(run);
	}

	let rotationTimer: ReturnType<typeof setInterval> | null = null;
	let loopTimer: ReturnType<typeof setInterval> | null = null;

	let currentDisplayText = '';
	let currentPhrases: string[] = [];

	$effect(() => {
		if (rotationTimer) {
			clearInterval(rotationTimer);
			rotationTimer = null;
		}
		if (loopTimer) {
			clearInterval(loopTimer);
			loopTimer = null;
		}

		const newPhrases = status?.phrases ?? [];
		const newActive = status?.active ?? active;
		const newText = status?.text ?? '';

		const targetText = newPhrases.length > 0 ? newPhrases[0] : newText;

		if (
			targetText === currentDisplayText &&
			newPhrases.length === currentPhrases.length &&
			newActive === !isComplete
		) {
			if (newActive && targetText && !loopTimer) {
				loopTimer = setInterval(() => {
					if (mounted && targetText) scrambleText(targetText);
				}, SCRAMBLE_LOOP_INTERVAL);
			}
			return;
		}

		currentDisplayText = targetText;
		currentPhrases = newPhrases;
		isComplete = !newActive;

		if (isComplete) return;

		if (targetText) scrambleText(targetText);

		if (newPhrases.length > 1) {
			rotationTimer = setInterval(() => {
				currentPhraseIndex = (currentPhraseIndex + 1) % newPhrases.length;
				if (mounted && newPhrases[currentPhraseIndex]) {
					scrambleText(newPhrases[currentPhraseIndex]);
				}
			}, ROTATION_INTERVAL);
		}

		if (newActive && targetText) {
			loopTimer = setInterval(() => {
				if (mounted && targetText) scrambleText(targetText);
			}, SCRAMBLE_LOOP_INTERVAL);
		}

		return () => {
			if (rotationTimer) {
				clearInterval(rotationTimer);
				rotationTimer = null;
			}
			if (loopTimer) {
				clearInterval(loopTimer);
				loopTimer = null;
			}
		};
	});

	onMount(() => {
		mounted = true;

		return () => {
			mounted = false;
			if (animationFrame !== 0) {
				cancelAnimationFrame(animationFrame);
				animationFrame = 0;
			}
			if (rotationTimer) clearInterval(rotationTimer);
			if (loopTimer) clearInterval(loopTimer);
		};
	});
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Agent status">
	<div
		class={[
			styles.panel,
			isStable && styles.stable,
			isComplete && styles.complete,
			styles[`tone-${dotTone}`]
		]
			.filter(Boolean)
			.join(' ')}
	>
		<span class={styles.dot} aria-hidden="true"></span>

		<span class={styles.label}>{displayText}</span>

		{#if elapsedLabel && !isComplete}
			<span class={styles.elapsed} aria-hidden="true">{elapsedLabel}</span>
		{/if}

		{#if status?.stallWarning}
			<span class={styles.stallNote}>
				{#if status.stallWarning === 'still-working'}
					still working
				{:else if status.stallWarning === 'taking-longer'}
					taking longer
				{:else}
					waiting on LLM
				{/if}
			</span>
		{/if}

		<span class={styles.sweep} aria-hidden="true"></span>
	</div>
</div>
