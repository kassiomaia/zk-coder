<script lang="ts">
	/**
	 * Input Component
	 * Form input with label, helper text, and validation states.
	 */
	import styles from './Input.module.css';

	interface Props {
		/** Input type */
		type?: 'text' | 'password' | 'email' | 'url';
		/** Input name */
		name?: string;
		/** Input value */
		value: string;
		/** Value change handler */
		onchange: (value: string) => void;
		/** Placeholder text */
		placeholder?: string;
		/** Label text */
		label?: string;
		/** Helper text below input */
		helper?: string;
		/** Error message */
		error?: string;
		/** Disabled state */
		disabled?: boolean;
		/** Required field */
		required?: boolean;
		/** Auto-focus on mount */
		autofocus?: boolean;
		/** Additional input class */
		inputClass?: string;
	}

	let {
		type = 'text',
		name,
		value,
		onchange,
		placeholder = '',
		label = '',
		helper = '',
		error = '',
		disabled = false,
		required = false,
		autofocus = false,
		inputClass = ''
	}: Props = $props();

	let inputRef: HTMLInputElement | undefined = $state();

	let inputClasses = $derived(
		[styles.input, error && styles.error, inputClass].filter(Boolean).join(' ')
	);

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		onchange(target.value);
	}

	$effect(() => {
		if (autofocus && inputRef) {
			inputRef.focus();
		}
	});
</script>

{#if label}
	<label class={styles.label} for={name}>
		{label}
		{#if required}
			<span class={styles.required}>*</span>
		{/if}
	</label>
{/if}
<input
	bind:this={inputRef}
	class={inputClasses}
	{type}
	{name}
	{value}
	{placeholder}
	{disabled}
	{required}
	oninput={handleInput}
	aria-invalid={!!error}
	aria-describedby={error ? `${name}-error` : helper ? `${name}-helper` : undefined}
/>
{#if error}
	<div id="{name}-error" class={[styles.message, styles.error].join(' ')} role="alert">
		{error}
	</div>
{:else if helper}
	<div id="{name}-helper" class={styles.message}>
		{helper}
	</div>
{/if}
