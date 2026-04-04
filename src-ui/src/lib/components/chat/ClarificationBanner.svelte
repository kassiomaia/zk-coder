<script lang="ts">
	/**
	 * ClarificationBanner Component
	 * Displays ambiguity resolution UI for user clarification.
	 * Shows options from ClarificationRequest and allows freeform input if enabled.
	 */
	import styles from './ClarificationBanner.module.css';
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import { AlertCircleIcon, CloseIcon, EditIcon, SendIcon } from '$lib/icons';
	import type { Ambiguity, ClarificationOption, ResolutionType } from '$lib/types/nlp';
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

	let { ambiguity, onResolve, onDismiss, class: className = '' }: Props = $props();

	// State for custom input
	let customInput = $state('');
	let showCustomInput = $state(false);
	let customInputRef: HTMLInputElement | undefined = $state();

	// Focus input when custom input is shown
	$effect(() => {
		if (showCustomInput && customInputRef) {
			customInputRef.focus();
		}
	});

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
			<AlertCircleIcon class={styles.icon} size={16} />
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
					bind:this={customInputRef}
					type="text"
					class={styles.customInput}
					bind:value={customInput}
					onkeydown={handleKeydown}
					placeholder="Enter custom value..."
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
							<SendIcon size={16} filled={true} />
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
							<CloseIcon size={16} />
						{/snippet}
					</IconButton>
				</div>
			</div>
		{:else}
			<button class={styles.customToggle} onclick={toggleCustomInput} type="button">
				<EditIcon size={14} />
				<span>Enter custom value</span>
			</button>
		{/if}
	{/if}

	<!-- Actions -->
	<div class={styles.actions}>
		<button class={styles.skipButton} onclick={handleSkip} type="button"> Skip </button>
	</div>
</div>
