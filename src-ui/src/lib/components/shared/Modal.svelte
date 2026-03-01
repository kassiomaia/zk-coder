<script lang="ts">
	/**
	 * Modal Component
	 * Reusable modal with overlay, header, body, and footer slots.
	 */
	import styles from './Modal.module.css';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Modal title */
		title?: string;
		/** Open state */
		open: boolean;
		/** Close handler */
		onclose: () => void;
		/** Size variant */
		size?: 'default' | 'lg';
		/** Show close button */
		showClose?: boolean;
		/** Additional class for modal content */
		contentClass?: string;
		/** Children */
		children?: Snippet;
		/** Footer snippet */
		footer?: Snippet;
	}

	let { title = '', open, onclose, size = 'default', showClose = true, contentClass = '', children, footer }: Props = $props();

	let modalClasses = $derived([
		styles.modal,
		styles[size],
		open && styles.open
	].filter(Boolean).join(' '));

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			onclose();
		}
	}

	$effect(() => {
		if (open) {
			document.addEventListener('keydown', handleKeydown);
			document.body.style.overflow = 'hidden';
		}
		return () => {
			document.removeEventListener('keydown', handleKeydown);
			document.body.style.overflow = '';
		};
	});
</script>

{#if open}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div class={styles.overlay} onclick={onclose} role="presentation"></div>
	<div class={modalClasses} role="dialog" aria-modal="true" aria-labelledby={title ? 'modal-title' : undefined}>
		{#if title || showClose}
			<header class={styles.header}>
				{#if title}
					<h2 id="modal-title" class={styles.title}>{title}</h2>
				{/if}
				{#if showClose}
					<button
						class={styles.closeBtn}
						onclick={onclose}
						type="button"
						aria-label="Close modal"
					>
						<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<line x1="18" y1="6" x2="6" y2="18"></line>
							<line x1="6" y1="6" x2="18" y2="18"></line>
						</svg>
					</button>
				{/if}
			</header>
		{/if}
		<div class={[styles.body, contentClass].filter(Boolean).join(' ')}>
			{#if children}
				{@render children()}
			{/if}
		</div>
		{#if footer}
			<footer class={styles.footer}>
				{@render footer()}
			</footer>
		{/if}
	</div>
{/if}

<style>
	/* Styles are in Modal.module.css */
</style>
