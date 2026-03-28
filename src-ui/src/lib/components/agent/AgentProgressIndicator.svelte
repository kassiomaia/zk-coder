<script lang="ts">
	/**
	 * AgentProgressIndicator Component
	 *
	 * Dynamic progress feedback showing real agent execution phases.
	 * Replaces static loading indicators with meaningful state communication.
	 *
	 * Phases:
	 * - analyzing: Understanding the request and context
	 * - planning: Creating execution plan
	 * - executing: Running the planned steps
	 * - validating: Verifying results
	 * - completed: Task finished successfully
	 * - error: Execution failed
	 */
	import styles from './AgentProgressIndicator.module.css';
	import { Brain, GitBranch, Cpu, CheckCircle, AlertCircle, Loader2 } from 'lucide-svelte';

	export type ProgressPhase =
		| 'analyzing'
		| 'planning'
		| 'executing'
		| 'validating'
		| 'completed'
		| 'error';

	export interface PhaseStep {
		id: string;
		phase: ProgressPhase;
		label: string;
		detail?: string;
		duration?: number;
	}

	interface Props {
		/** Current active phase */
		currentPhase: ProgressPhase;
		/** Optional detail message for current phase */
		detail?: string;
		/** Total steps for progress bar */
		totalSteps?: number;
		/** Current step number */
		currentStep?: number;
		/** Whether to show compact variant */
		compact?: boolean;
		/** Error message if in error state */
		errorMessage?: string;
	}

	let {
		currentPhase,
		detail = '',
		totalSteps,
		currentStep = 1,
		compact = false,
		errorMessage = ''
	}: Props = $props();

	// Phase configuration
	const phases: Record<ProgressPhase, { icon: typeof Brain; label: string; description: string }> = {
		analyzing: {
			icon: Brain,
			label: 'Analyzing',
			description: 'Understanding your request and gathering context'
		},
		planning: {
			icon: GitBranch,
			label: 'Planning',
			description: 'Creating an execution plan'
		},
		executing: {
			icon: Cpu,
			label: 'Executing',
			description: 'Running planned steps'
		},
		validating: {
			icon: CheckCircle,
			label: 'Validating',
			description: 'Verifying results'
		},
		completed: {
			icon: CheckCircle,
			label: 'Completed',
			description: 'Task finished successfully'
		},
		error: {
			icon: AlertCircle,
			label: 'Error',
			description: 'Execution failed'
		}
	};

	// Calculate progress percentage
	let progress = $derived(() => {
		if (!totalSteps || currentPhase === 'completed') return 100;
		if (currentPhase === 'error') return 0;
		return Math.min(100, Math.round((currentStep / totalSteps) * 100));
	});

	// Get phase index for visual ordering
	let phaseIndex = $derived(() => {
		const order: ProgressPhase[] = ['analyzing', 'planning', 'executing', 'validating', 'completed'];
		if (currentPhase === 'error') return -1;
		return order.indexOf(currentPhase);
	});

	// Check if phase is active
	function isPhaseActive(phase: ProgressPhase): boolean {
		return phase === currentPhase;
	}

	// Check if phase is completed
	function isPhaseCompleted(phase: ProgressPhase): boolean {
		const order: ProgressPhase[] = ['analyzing', 'planning', 'executing', 'validating', 'completed'];
		const currentIndex = order.indexOf(currentPhase);
		const phaseIdx = order.indexOf(phase);
		if (currentPhase === 'error') return false;
		return phaseIdx < currentIndex;
	}

	// Get phase state class
	function getPhaseClass(phase: ProgressPhase): string {
		if (isPhaseActive(phase)) return styles.active;
		if (isPhaseCompleted(phase)) return styles.completed;
		return styles.pending;
	}
</script>

<div
	class={[
		styles.indicator,
		compact && styles.compact,
		currentPhase === 'error' && styles.error
	].filter(Boolean).join(' ')}
	aria-live="polite"
	aria-busy={currentPhase !== 'completed' && currentPhase !== 'error'}
>
	{#if compact}
		<!-- Compact variant: single line with icon and text -->
		<div class={styles.compactView}>
			<div class={styles.compactIcon}>
				{#if currentPhase === 'completed'}
					<CheckCircle size={16} strokeWidth={2} />
				{:else if currentPhase === 'error'}
					<AlertCircle size={16} strokeWidth={2} />
				{:else}
					<Loader2 size={16} strokeWidth={2} class={styles.spinning} />
				{/if}
			</div>
			<span class={styles.compactLabel}>
				{phases[currentPhase].label}
				{#if detail}
					<span class={styles.compactDetail}> · {detail}</span>
				{/if}
			</span>
			{#if totalSteps && currentPhase !== 'completed' && currentPhase !== 'error'}
				<span class={styles.compactProgress}>Step {currentStep} of {totalSteps}</span>
			{/if}
		</div>
	{:else}
		<!-- Full variant: detailed phase progress -->
		<div class={styles.phaseList}>
			{#each Object.entries(phases) as [phase, config]}
				{#if phase !== 'completed' && phase !== 'error'}
					<div
						class={[styles.phase, getPhaseClass(phase as ProgressPhase)].join(' ')}
					>
						<div class={styles.phaseIcon}>
							<svelte:component this={config.icon} size={14} strokeWidth={2} />
						</div>
						<div class={styles.phaseInfo}>
							<span class={styles.phaseLabel}>{config.label}</span>
							{#if isPhaseActive(phase as ProgressPhase) && detail}
								<span class={styles.phaseDetail}>{detail}</span>
							{:else if isPhaseActive(phase as ProgressPhase)}
								<span class={styles.phaseDetail}>{config.description}</span>
							{/if}
						</div>
						{#if isPhaseCompleted(phase as ProgressPhase)}
							<CheckCircle size={14} strokeWidth={2} class={styles.phaseCheck} />
						{/if}
					</div>
				{/if}
			{/each}
		</div>

		<!-- Progress bar -->
		{#if totalSteps && currentPhase !== 'completed' && currentPhase !== 'error'}
			<div class={styles.progressContainer}>
				<div class={styles.progressTrack}>
					<div
						class={styles.progressFill}
						style:width={`${progress()}%`}
						role="progressbar"
						aria-valuenow={currentStep}
				aria-valuemin={0}
				aria-valuemax={totalSteps}
					></div>
				</div>
				<span class={styles.progressLabel}>
					Step {currentStep} of {totalSteps}
				</span>
			</div>
		{/if}

		<!-- Error message -->
		{#if currentPhase === 'error' && errorMessage}
			<div class={styles.errorContainer}>
				<AlertCircle size={14} strokeWidth={2} />
				<span class={styles.errorMessage}>{errorMessage}</span>
			</div>
		{/if}
	{/if}
</div>
