<script lang="ts">
	/**
	 * ClarificationBanner Component
	 * Displays ambiguity resolution UI for user clarification.
	 * Shows options from ClarificationRequest and allows freeform input if enabled.
	 */
	import styles from './ClarificationBanner.module.css';
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import type {
		Ambiguity,
		ClarificationOption,
		ResolutionType
	} from '$lib/types/nlp';
	import { AMBIGUITY_TYPE_LABELS } from '$lib/types/nlp';

	interface Props {
		/** The ambiguity to display */
		ambiguity: Ambiguity;
		/** Handler when user resolves the ambiguity */
		onResolve: (ambiguityId: string, resolution: string, type: ResolutionType) => void;
		/** Handler when user dismisses the ambiguity */
		onDismiss: (ambiguityId: string) => void;
		/** Optional CSS class */
		class?: string;
	}

	let {
		ambiguity,
		onResolve,
		onDismiss,
		class: className = ''
	}: Props = $props();

	// State for custom input
	let customInput = $state('');
	let showCustomInput = $state(false);

	// Derived values
	let severityClass = $derived(styles[ambiguity.severity]);
	let typeLabel = $derived(AMBIGUITY_TYPE_LABELS[ambiguity.ambiguity_type]);
	let hasOptions = $derived(ambiguity.clarification.options.length > 0);
	let allowsFreeform = $derived(ambiguity.clarification.allows_freeform);

	function handleOptionSelect(option: ClarificationOption) {
		onResolve(ambiguity.id, option.value, 'selected');
	}

	function handleCustomSubmit() {
		if (customInput.trim()) {
			onResolve(ambiguity.id, customInput.trim(), 'custom');
			customInput = '';
		}
	}

	function handleSkip() {
		onDismiss(ambiguity.id);
	}

	function toggleCustomInput() {
		showCustomInput = !showCustomInput;
		if (!showCustomInput) {
			customInput = '';
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleCustomSubmit();
		}
		if (e.key === 'Escape') {
			showCustomInput = false;
			customInput = '';
		}
	}
</script>

<div class={[styles.banner, severityClass, className].filter(Boolean).join(' ')}>
	<!-- Header with type and question -->
	<div class={styles.header}>
		<div class={styles.iconWrapper}>
			<svg
				class={styles.icon}
				width="16"
				height="16"
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
		</div>
		<div class={styles.headerText}>
			<span class={styles.typeLabel}>{typeLabel}</span>
			<span class={styles.question}>{ambiguity.clarification.question}</span>
		</div>
	</div>

	<!-- Options -->
	{#if hasOptions}
		<div class={styles.options}>
			{#each ambiguity.clarification.options as option (option.value)}
				<button
					class={styles.optionButton}
					onclick={() => handleOptionSelect(option)}
					type="button"
				>
					<span class={styles.optionLabel}>{option.label}</span>
					{#if option.description}
						<span class={styles.optionDescription}>{option.description}</span>
					{/if}
				</button>
			{/each}
		</div>
	{/if}

	<!-- Custom input section -->
	{#if allowsFreeform}
		{#if showCustomInput}
			<div class={styles.customInputWrapper}>
				<input
					type="text"
					class={styles.customInput}
					bind:value={customInput}
					onkeydown={handleKeydown}
					placeholder="Enter custom value..."
					autofocus
				/>
				<div class={styles.customInputActions}>
					<IconButton
						label="Submit custom value"
						tooltip="Submit"
						variant="subtle"
						intent="success"
						size="sm"
						disabled={!customInput.trim()}
						onclick={handleCustomSubmit}
					>
						{#snippet icon()}
							<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
								<path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
							</svg>
						{/snippet}
					</IconButton>
					<IconButton
						label="Cancel custom input"
						tooltip="Cancel"
						variant="ghost"
						size="sm"
						onclick={toggleCustomInput}
					>
						{#snippet icon()}
							<svg
								width="16"
								height="16"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<line x1="18" y1="6" x2="6" y2="18" />
								<line x1="6" y1="6" x2="18" y2="18" />
							</svg>
						{/snippet}
					</IconButton>
				</div>
			</div>
		{:else}
			<button class={styles.customToggle} onclick={toggleCustomInput} type="button">
				<svg
					width="14"
					height="14"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
					<path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
				</svg>
				<span>Enter custom value</span>
			</button>
		{/if}
	{/if}

	<!-- Actions -->
	<div class={styles.actions}>
		<button class={styles.skipButton} onclick={handleSkip} type="button">
			Skip
		</button>
	</div>
</div>
