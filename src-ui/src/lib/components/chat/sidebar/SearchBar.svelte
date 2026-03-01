<script lang="ts">
	/**
	 * SearchBar Component
	 * Search input for filtering conversations.
	 */
	import styles from './SearchBar.module.css';

	interface Props {
		/** Current search value */
		value?: string;
		/** Input handler */
		oninput?: (value: string) => void;
		/** Placeholder text */
		placeholder?: string;
		/** Disabled state */
		disabled?: boolean;
	}

	let { value = '', oninput, placeholder = 'Search...', disabled = false }: Props = $props();

	let inputRef: HTMLInputElement | undefined = $state();

	function handleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		oninput?.(target.value);
	}

	function clearSearch() {
		oninput?.('');
		inputRef?.focus();
	}
</script>

<div class={styles.search}>
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
		aria-hidden="true"
	>
		<circle cx="11" cy="11" r="8" />
		<line x1="21" y1="21" x2="16.65" y2="16.65" />
	</svg>

	<input
		bind:this={inputRef}
		class={styles.input}
		type="search"
		{value}
		{placeholder}
		{disabled}
		oninput={handleInput}
		aria-label="Search conversations"
	/>

	{#if value}
		<button
			class={styles.clear}
			onclick={clearSearch}
			aria-label="Clear search"
			type="button"
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<line x1="18" y1="6" x2="6" y2="18" />
				<line x1="6" y1="6" x2="18" y2="18" />
			</svg>
		</button>
	{/if}
</div>
