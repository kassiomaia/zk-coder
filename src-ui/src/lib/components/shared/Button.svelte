<script lang="ts">
	/**
	 * Button Component
	 * Primary, secondary, ghost, and danger variants.
	 */
	import styles from './Button.module.css';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Button variant */
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		/** Button size */
		size?: 'default' | 'sm';
		/** Disabled state */
		disabled?: boolean;
		/** Loading state */
		loading?: boolean;
		/** Icon only */
		iconOnly?: boolean;
		/** Click handler */
		onclick?: () => void;
		/** Type attribute */
		type?: 'button' | 'submit' | 'reset';
		/** Icon snippet */
		icon?: Snippet;
		/** Children */
		children?: Snippet;
	}

	let {
		variant = 'primary',
		size = 'default',
		disabled = false,
		loading = false,
		iconOnly = false,
		onclick,
		type = 'button',
		icon,
		children
	}: Props = $props();

	let classes = $derived([
		styles.btn,
		styles[variant],
		styles[size],
		disabled && styles.disabled,
		loading && styles.loading,
		iconOnly && styles.iconOnly
	].filter(Boolean).join(' '));

	function handleClick() {
		if (!disabled && !loading && onclick) {
			onclick();
		}
	}
</script>

<button
	class={classes}
	{disabled}
	{type}
	onclick={handleClick}
	aria-disabled={disabled || loading}
>
	{#if loading}
		<svg class={styles.spinner} xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<circle cx="12" cy="12" r="10" stroke-opacity="0.25"></circle>
			<path d="M12 2a10 10 0 0 1 10 10" stroke-opacity="0.75">
				<animateTransform attributeName="transform" type="rotate" from="0 12 12" to="360 12 12" dur="1s" repeatCount="indefinite"/>
			</path>
		</svg>
	{:else if icon}
		{@render icon()}
	{/if}
	{#if !iconOnly && children}
		{@render children()}
	{/if}
</button>

<style>
	/* Styles are in Button.module.css */
</style>
