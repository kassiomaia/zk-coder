<script lang="ts">
    /**
     * AnsweringIndicator Component
     * Shows agent execution status with elegant letter-shuffle animation.
     * Displays real-time status from agent events or fallback phases.
     */
    import {onMount} from 'svelte';
    import {CheckCircle, Clock, Edit, File, Scan, Sparkles, Terminal} from 'lucide-svelte';
    import styles from './AnsweringIndicator.module.css';
    import type {AgentStatus} from '$lib/utils/agentStatus';

    const SCRAMBLE_DURATION = 800; // Duration for the letter scramble animation in milliseconds

	const SCRAMBLE_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_';

	interface Props {
		/** Current agent status text (if not provided, shows default phases) */
		status?: AgentStatus | null;
		/** Whether the indicator is active/animating */
		active?: boolean;
	}

	let { status, active = true }: Props = $props();

	// Animation configuration
	const SCRAMBLE_LOOP_INTERVAL = 3000; // Re-scramble same text every 3 seconds

	// Phrase rotation state
	let currentPhraseIndex = 0;

	// Phrase rotation configuration
	const ROTATION_INTERVAL = 2500; // Rotate every 2.5 seconds

	// Icon mapping for status types
	const iconMap = {
		scan: Scan,
		terminal: Terminal,
		file: File,
		edit: Edit,
		wait: Clock,
		check: CheckCircle,
		sparkle: Sparkles
	} as const;

	// Current display state
	let displayText = $state('Starting...');
	let isStable = $state(true);
	let isComplete = $state(false);

	// Animation state
	let animationFrame: number;
	let mounted = false;

	// Get the current icon type based on status
	let currentIconType = $derived(
		status?.icon ?? 'sparkle'
	);

	// Damped easing function (easeOutQuart) for smooth resolution
	function easeOutQuart(x: number): number {
		return 1 - Math.pow(1 - x, 4);
	}

	function scrambleText(target: string) {
		if (displayText === target) return;
		
		isStable = false;
		let startTime: number;
		const startText = displayText;
		const targetLength = target.length;
		const startLength = startText.length;
		const maxLength = Math.max(startLength, targetLength);

		const run = (now: number) => {
			if (!mounted) return;
			if (startTime === undefined) startTime = now;

			const elapsed = now - startTime;
			const progress = Math.min(elapsed / SCRAMBLE_DURATION, 1);
			const easedProgress = easeOutQuart(progress);

			// Calculate how many characters should be resolved based on eased progress
			const resolvedCount = Math.floor(easedProgress * targetLength);

			let current = '';
			for (let i = 0; i < maxLength; i++) {
				if (i < resolvedCount) {
					// Character from target resolved
					current += target[i];
				} else if (i < targetLength) {
					// Character from target still scrambling
					const seed = i + Math.floor(elapsed / 50);
					current += SCRAMBLE_CHARS[seed % SCRAMBLE_CHARS.length];
				} else if (i < startLength) {
					// Character from previous text still exists (shrinking)
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
			}
		};

		animationFrame = requestAnimationFrame(run);
	}

	// Watch for status changes and animate the transition
	// Also handles phrase rotation and continuous loop animation
	let rotationTimer: ReturnType<typeof setInterval> | null = null;
	let loopTimer: ReturnType<typeof setInterval> | null = null;

	// Track the actual values that matter, not object references
	let currentDisplayText = '';
	let currentPhrases: string[] = [];

	$effect(() => {
		// Clear any existing timers
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

		// Determine what to display
		const targetText = newPhrases.length > 0 ? newPhrases[0] : newText;

		// Check if anything meaningful changed
		if (targetText === currentDisplayText && newPhrases.length === currentPhrases.length && newActive === !isComplete) {
			// Nothing meaningful changed, but we still need to keep the loop running if active
			if (newActive && targetText && !loopTimer) {
				// Restart the loop timer
				loopTimer = setInterval(() => {
					if (mounted && targetText) {
						scrambleText(targetText);
					}
				}, SCRAMBLE_LOOP_INTERVAL);
			}
			return;
		}

		// Update tracked values
		currentDisplayText = targetText;
		currentPhrases = newPhrases;
		isComplete = !newActive;

		if (isComplete) return;

		// Initial animation
		if (targetText) {
			scrambleText(targetText);
		}

		// Start phrase rotation if we have multiple phrases
		if (newPhrases.length > 1) {
			// Set up rotation timer - rotate every 2.5 seconds
			rotationTimer = setInterval(() => {
				currentPhraseIndex = (currentPhraseIndex + 1) % newPhrases.length;
				if (mounted && newPhrases[currentPhraseIndex]) {
					scrambleText(newPhrases[currentPhraseIndex]);
				}
			}, 2500);
		}

		// Always run the continuous loop animation when active
		// This keeps the scramble effect running even with a single phrase
		if (newActive && targetText) {
			loopTimer = setInterval(() => {
				if (mounted && targetText) {
					scrambleText(targetText);
				}
			}, SCRAMBLE_LOOP_INTERVAL);
		}

		// Cleanup: clear timers when effect reruns or component unmounts
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
			cancelAnimationFrame(animationFrame);
			if (rotationTimer) {
				clearInterval(rotationTimer);
			}
		};
	});
</script>

<div class={styles.container} role="status" aria-live="polite" aria-label="Agent status">
	<div
		class={[styles.panel, isStable && styles.stable, isComplete && styles.complete]
			.filter(Boolean)
			.join(' ')}
	>
		<div class={styles.iconWrapper}>
			{#if currentIconType === 'scan'}
				<Scan size={16} class={styles.icon} />
			{:else if currentIconType === 'terminal'}
				<Terminal size={16} class={styles.icon} />
			{:else if currentIconType === 'file'}
				<File size={16} class={styles.icon} />
			{:else if currentIconType === 'edit'}
				<Edit size={16} class={styles.icon} />
			{:else if currentIconType === 'wait'}
				<Clock size={16} class={styles.icon} />
			{:else if currentIconType === 'check'}
				<CheckCircle size={16} class={styles.icon} />
			{:else}
				<Sparkles size={16} class={styles.icon} />
			{/if}
		</div>

		<div class={styles.labelContainer}>
			<span class={styles.label}>{displayText}</span>
			<div class={styles.activityIndicator}></div>
			{#if status?.stallWarning && status.elapsedSeconds}
				<span class={styles.stallWarning}>
					{#if status.stallWarning === 'still-working'}
						· Still working...
					{:else if status.stallWarning === 'taking-longer'}
						· Taking longer than expected ({Math.floor(status.elapsedSeconds / 60)}:{(status.elapsedSeconds % 60).toString().padStart(2, '0')})
					{:else if status.stallWarning === 'very-long'}
						· Still waiting for LLM ({Math.floor(status.elapsedSeconds / 60)}:{(status.elapsedSeconds % 60).toString().padStart(2, '0')})
					{/if}
				</span>
			{/if}
		</div>
	</div>
</div>
