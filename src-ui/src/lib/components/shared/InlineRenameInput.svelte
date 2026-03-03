<script lang="ts">
	/**
	 * InlineRenameInput Component
	 * Enhanced input for renaming items with validation, visual feedback, and microcopy.
	 */
	import { onMount } from 'svelte';
	import { fade, scale } from 'svelte/transition';
	import { cubicOut } from 'svelte/easing';
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
	}

	let { value = '', placeholder = '', maxLength = 50, onSave, onCancel }: Props = $props();

	let inputValue = $state(value);
	let inputRef: HTMLInputElement | undefined = $state();
	let error = $state('');
	let isDirty = $derived(inputValue !== value);
	let charCount = $derived(inputValue.length);
	let remainingChars = $derived(maxLength - charCount);

	function validate() {
		if (inputValue.trim().length === 0) {
			error = 'Name cannot be empty';
			return false;
		}
		if (inputValue.length > maxLength) {
			error = 'Name too long';
			return false;
		}
		error = '';
		return true;
	}

	function handleSave() {
		if (validate()) {
			if (isDirty) {
				onSave(inputValue.trim());
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
	}

	onMount(() => {
		if (inputRef) {
			inputRef.focus();
			inputRef.select();
		}
	});
</script>

<div
	class={styles.container}
	in:scale={{ duration: 200, easing: cubicOut, start: 0.98, opacity: 0 }}
	out:fade={{ duration: 150 }}
>
	<div class={styles.inputWrapper}>
		<input
			bind:this={inputRef}
			class={[styles.input, error && styles.hasError].filter(Boolean).join(' ')}
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
		<div class={styles.indicators}>
			{#if remainingChars <= 10}
				<span class={[styles.charCount, remainingChars < 0 && styles.errorText].filter(Boolean).join(' ')}>
					{remainingChars}
				</span>
			{/if}
		</div>
	</div>

	<div class={styles.footer}>
		{#if error}
			<span class={styles.errorMessage} transition:fade>{error}</span>
		{:else}
			<span class={styles.hint} transition:fade>Enter to save • Esc to cancel</span>
		{/if}
	</div>
</div>
