<script lang="ts">
	/**
	 * AnsweringIndicator Component
	 * Displays a structured activity panel with letter-level shuffle transitions.
	 */
	import { onMount, onDestroy } from 'svelte';
	import { Scan, Layout, Sparkles, ShieldCheck, CheckCircle } from 'lucide-svelte';
	import styles from './AnsweringIndicator.module.css';

	// Optimization: Equal length phrases (17 chars) to preserve rhythm
	const phases = [
		{ label: 'Analyzing Context', icon: Scan },
		{ label: 'Building Response', icon: Layout },
		{ label: 'Refining Content ', icon: Sparkles },
		{ label: 'Checking Accuracy', icon: ShieldCheck },
		{ label: 'Finalizing Output', icon: CheckCircle }
	];

	const SCRAMBLE_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_';
	const PHASE_DURATION = 2500; // Total time per phase
	const SCRAMBLE_SPEED = 40; // ms per frame for scramble
	
	let currentPhaseIndex = 0;
	let displayText = $state(phases[0].label);
	let isStable = $state(true); // Start stable
	let interval: ReturnType<typeof setInterval>;
	let animationFrame: number;

	function scrambleText(target: string, onComplete: () => void) {
		isStable = false;
		let iteration = 0;
		let currentText = displayText;
		const maxLength = Math.max(currentText.length, target.length);
		
		const run = () => {
			displayText = target
				.split('')
				.map((letter, index) => {
					if (index < iteration) {
						return target[index];
					}
					return SCRAMBLE_CHARS[Math.floor(Math.random() * SCRAMBLE_CHARS.length)];
				})
				.join('');

			if (iteration >= maxLength) {
				displayText = target;
				isStable = true;
				onComplete();
			} else {
				iteration += 1 / 3; // Controls the speed of resolution (resolve 1 char every 3 frames)
				animationFrame = requestAnimationFrame(run);
			}
		};

		run();
	}

	function nextPhase() {
		const nextIndex = (currentPhaseIndex + 1) % phases.length;
		currentPhaseIndex = nextIndex;
		
		scrambleText(phases[nextIndex].label, () => {
			// Schedule next cycle after stabilization
			interval = setTimeout(nextPhase, PHASE_DURATION);
		});
	}

	onMount(() => {
		// Start the cycle after initial delay
		interval = setTimeout(nextPhase, PHASE_DURATION);

		return () => {
			clearTimeout(interval);
			cancelAnimationFrame(animationFrame);
		};
	});
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Assistant is answering">
	<div class={[styles.panel, isStable && styles.stable].filter(Boolean).join(' ')}>
		<div class={styles.iconWrapper}>
			<svelte:component
				this={phases[currentPhaseIndex].icon}
				size={14}
				class={styles.icon}
			/>
		</div>
		
		<div class={styles.labelContainer}>
			<span class={styles.label}>{displayText}</span>
		</div>
		
		<!-- Highlight sweep only active when stable -->
		<div class={styles.highlightSweep}></div>
	</div>
</div>
