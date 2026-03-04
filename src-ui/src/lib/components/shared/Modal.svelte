<script lang="ts">
	/**
	 * Modal Component
	 * Reusable modal with overlay wrapper, header, body, and footer slots.
	 * Matches the prototype pattern: .modal-overlay wraps .modal
	 */
	import styles from './Modal.module.css';
	import IconButton from './IconButton.svelte';
	import type { Snippet } from 'svelte';

	interface Props {
		/** Modal title */
		title?: string;
		/** Open state */
		open: boolean;
		/** Close handler */
		onclose: () => void;
		/** Size variant */
		size?: 'default' | 'lg' | 'xl';
		/** Show close button */
		showClose?: boolean;
		/** Additional class for modal content */
		contentClass?: string;
		/** Stacked modal - higher z-index for modals opened from other modals */
		stacked?: boolean;
		/** ID of element that describes the modal content */
		describedby?: string;
		/** Children */
		children?: Snippet;
		/** Footer snippet */
		footer?: Snippet;
	}

	let {
		title = '',
		open,
		onclose,
		size = 'default',
		showClose = true,
		contentClass = '',
		stacked = false,
		describedby,
		children,
		footer
	}: Props = $props();

	let wrapperClasses = $derived(
		[styles.overlay, stacked && styles.top, open && styles.open].filter(Boolean).join(' ')
	);

	let modalClasses = $derived(
		[styles.modal, styles[size], open && styles.open].filter(Boolean).join(' ')
	);

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && open) {
			onclose();
		}
	}

	// Prevent body scroll when modal is open
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
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class={wrapperClasses}
		onclick={(e) => {
			// Close only when clicking the overlay, not the modal content
			if (e.target === e.currentTarget) onclose();
		}}
		onkeydown={(e) => {
			// Allow closing with Escape key
			if (e.key === 'Escape') onclose();
		}}
		role="presentation"
	>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div
			class={modalClasses}
			role="dialog"
			aria-modal="true"
			aria-labelledby={title ? 'modal-title' : undefined}
			aria-describedby={describedby}
			tabindex="-1"
			onclick={(e) => e.stopPropagation()}
		>
			{#if title || showClose}
				<header class={styles.header}>
					{#if title}
						<h2 id="modal-title" class={styles.title}>{title}</h2>
					{/if}
					{#if showClose}
						<IconButton
							label="Close modal"
							tooltip="Close"
							variant="ghost"
							size="sm"
							onclick={onclose}
						>
							{#snippet icon()}
								<svg
									xmlns="http://www.w3.org/2000/svg"
									width="20"
									height="20"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									stroke-linecap="round"
									stroke-linejoin="round"
								>
									<line x1="18" y1="6" x2="6" y2="18"></line>
									<line x1="6" y1="6" x2="18" y2="18"></line>
								</svg>
							{/snippet}
						</IconButton>
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
	</div>
{/if}

<style>
	/* Styles are in Modal.module.css */
</style>
