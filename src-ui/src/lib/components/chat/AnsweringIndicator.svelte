<script lang="ts">
	/**
	 * AnsweringIndicator Component
	 * Redesigned to follow a structured, Claude-style activity panel with 
	 * deterministic letter-shuffle transformations and damped easing.
	 */
	import { onMount, onDestroy } from 'svelte';
	import { Scan, Layout, Sparkles, ShieldCheck, CheckCircle } from 'lucide-svelte';
	import styles from './AnsweringIndicator.module.css';

	// User-defined phases with action-oriented microcopy
	const phases = [
		{ label: 'Analyzing Context', icon: Scan, intensity: 0.6 },
		{ label: 'Structuring Response', icon: Layout, intensity: 0.7 },
		{ label: 'Refining Clarity', icon: Sparkles, intensity: 0.8 },
		{ label: 'Validating Logic', icon: ShieldCheck, intensity: 0.9 },
		{ label: 'Finalizing Output', icon: CheckCircle, intensity: 1.0 }
	];

	const SCRAMBLE_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_';
	// Standardize duration scaling
	const PHASE_DURATION = 2200; 
	const SCRAMBLE_DURATION = 800; // Time to resolve text
	
	let currentPhaseIndex = $state(0);
	let displayText = $state(phases[0].label);
	let isStable = $state(true);
	let isComplete = $state(false);
	
	// Single-state animation controller variables
	let animationFrame: number;
	let timeoutId: ReturnType<typeof setTimeout>;
	let mounted = false;

	// Damped easing function (easeOutQuart) for smooth resolution
	function easeOutQuart(x: number): number {
		return 1 - Math.pow(1 - x, 4);
	}

	function scrambleText(target: string, onComplete: () => void) {
		isStable = false;
		let startTime: number;
		const startText = displayText;
		const maxLength = Math.max(startText.length, target.length);
		
		const run = (now: number) => {
			if (!mounted) return;
			if (startTime === undefined) startTime = now;

			const elapsed = now - startTime;
			const progress = Math.min(elapsed / SCRAMBLE_DURATION, 1);
			const easedProgress = easeOutQuart(progress);
			
			// Calculate how many characters should be resolved based on eased progress
			const resolvedCount = Math.floor(easedProgress * target.length);

			displayText = target
				.split('')
				.map((letter, index) => {
					if (index < resolvedCount) {
						return target[index];
					}
					// Deterministic-ish shuffle: use index and elapsed to pick char
					// to avoid "parallel rendering artifacts" (flickering of same char in same spot)
					const seed = index + Math.floor(elapsed / 50);
					return SCRAMBLE_CHARS[seed % SCRAMBLE_CHARS.length];
				})
				.join('');

			if (progress < 1) {
				animationFrame = requestAnimationFrame(run);
			} else {
				displayText = target;
				isStable = true;
				onComplete();
			}
		};

		animationFrame = requestAnimationFrame(run);
	}

	function runPhaseSequence(index: number) {
		if (index >= phases.length) {
			// Sequence complete
			isComplete = true;
			return;
		}

		currentPhaseIndex = index;
		
		scrambleText(phases[index].label, () => {
			if (!mounted) return;
			
			// Wait before next phase, unless it's the last one
			if (index < phases.length - 1) {
				timeoutId = setTimeout(() => {
					runPhaseSequence(index + 1);
				}, PHASE_DURATION - SCRAMBLE_DURATION); // Balance the total duration
			} else {
				// Final phase stabilization
				isComplete = true;
			}
		});
	}

	onMount(() => {
		mounted = true;
		// Start the sequence
		runPhaseSequence(0);

		return () => {
			mounted = false;
			cancelAnimationFrame(animationFrame);
			clearTimeout(timeoutId);
		};
	});
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Assistant is answering">
	<!-- 
		Panel container with dynamic intensity class for accent color 
		and completion state for the success pulse 
	-->
	<div 
		class={[
			styles.panel, 
			isStable && styles.stable, 
			isComplete && styles.complete
		].filter(Boolean).join(' ')}
		style="--intensity: {phases[currentPhaseIndex].intensity}"
	>
		<div class={styles.iconWrapper}>
			<svelte:component
				this={phases[currentPhaseIndex].icon}
				size={16}
				class={styles.icon}
			/>
		</div>
		
		<div class={styles.labelContainer}>
			<span class={styles.label}>{displayText}</span>
		</div>
	</div>
</div>
