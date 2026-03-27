<script lang="ts">
    /**
     * InlineRenameInput Component
     * Refined inline editing with morph transitions, subtle validation,
     * and optimistic update feedback.
     */
    import {onMount} from 'svelte';
    import {fade} from 'svelte/transition';
    import {cubicOut} from 'svelte/easing';
    import {AlertCircleIcon} from '$lib/icons';
    import styles from './InlineRenameInput.module.css';

    interface Props {
		/** Current value of the input */
		value: string;
		/** Placeholder text */
		placeholder?: string;
		/** Max length of the input */
		maxLength?: number;
		/** Save callback */
		onSave: (value: string) => void;
		/** Cancel callback */
		onCancel: () => void;
		/** Optional icon to display before input */
		icon?: import('svelte').Snippet;
	}

	let { value = '', placeholder = '', maxLength = 50, onSave, onCancel, icon }: Props = $props();

	let inputValue = $state('');
	let inputRef: HTMLInputElement | undefined = $state();

	// Sync inputValue with value prop when not editing
	$effect(() => {
		inputValue = value;
	});
	let error = $state('');
	let isValidating = $state(false);
	let showSaveConfirmation = $state(false);
	let isDirty = $derived(inputValue !== value);
	let charCount = $derived(inputValue.length);
	let remainingChars = $derived(maxLength - charCount);

	function morph(node: Element, { duration = 200 }) {
		return {
			duration,
			css: (t: number) => {
				const eased = cubicOut(t);
				// Scale 1.00 -> 1.01 -> 1.00
				const scale = 1 + Math.sin(t * Math.PI) * 0.01;
				return `
					opacity: ${eased};
					transform: scale(${scale});
				`;
			}
		};
	}

	function validate(): boolean {
		isValidating = true;

		if (inputValue.trim().length === 0) {
			error = 'Name cannot be empty';
			return false;
		}
		if (inputValue.length > maxLength) {
			error = `Maximum ${maxLength} characters`;
			return false;
		}

		error = '';
		return true;
	}

	function handleSave() {
		if (validate()) {
			if (isDirty) {
				// Trigger save confirmation animation
				showSaveConfirmation = true;
				setTimeout(() => {
					onSave(inputValue.trim());
				}, 400); // Matches --edit-sweep-duration roughly
			} else {
				onCancel();
			}
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			handleSave();
		} else if (e.key === 'Escape') {
			e.preventDefault();
			onCancel();
		}
		// Clear error on any input
		if (error) {
			error = '';
		}
	}

	onMount(() => {
		if (inputRef) {
			inputRef.focus();
			inputRef.select();
		}
	});
</script>

<div
	class={[styles.container, error && styles.hasError, showSaveConfirmation && styles.saveConfirming]
		.filter(Boolean)
		.join(' ')}
	in:morph={{ duration: 200 }}
	out:morph={{ duration: 200 }}
>
	{#if icon}
		<div class={styles.iconWrapper}>
			{@render icon()}
		</div>
	{/if}

	<div class={styles.contentColumn}>
		<input
			bind:this={inputRef}
			class={[styles.input, error && styles.inputError].filter(Boolean).join(' ')}
			type="text"
			bind:value={inputValue}
			{placeholder}
			maxlength={maxLength}
			onkeydown={handleKeydown}
			onblur={handleSave}
			oninput={() => {
				if (error) validate();
			}}
		/>

		<div class={styles.footer}>
			{#if error}
				<div class={styles.errorWrapper} in:fade={{ duration: 100 }}>
					<AlertCircleIcon size={12} class={styles.errorIcon} />
					<span class={styles.errorText}>{error}</span>
				</div>
			{:else}
				<span class={styles.hint}>Enter to save · Esc to cancel</span>
			{/if}

			{#if remainingChars <= 10}
				<span
					class={[styles.charCount, remainingChars <= 0 && styles.charCountError]
						.filter(Boolean)
						.join(' ')}
				>
					{remainingChars}
				</span>
			{/if}
		</div>
	</div>

	<!-- Highlight sweep overlay for save confirmation -->
	{#if showSaveConfirmation}
		<div class={styles.sweepOverlay}></div>
	{/if}
</div>
