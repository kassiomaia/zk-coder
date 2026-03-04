<script lang="ts">
	/**
	 * ConfirmDialog Component
	 * A simple confirmation dialog with confirm and cancel buttons.
	 * Used for destructive or important actions like archive, delete, etc.
	 */
	import Modal from './Modal.svelte';
	import Button from './Button.svelte';
	import styles from './ConfirmDialog.module.css';

	interface Props {
		/** Open state */
		open: boolean;
		/** Dialog title */
		title: string;
		/** Confirmation message */
		message: string;
		/** Confirm button label */
		confirmLabel?: string;
		/** Cancel button label */
		cancelLabel?: string;
		/** Confirm button variant */
		variant?: 'primary' | 'danger' | 'warning';
		/** Called when confirmed */
		onconfirm: () => void;
		/** Called when cancelled */
		oncancel: () => void;
		/** Loading state */
		loading?: boolean;
	}

	let {
		open,
		title,
		message,
		confirmLabel = 'Confirm',
		cancelLabel = 'Cancel',
		variant = 'primary',
		onconfirm,
		oncancel,
		loading = false
	}: Props = $props();

	function handleConfirm() {
		if (!loading) {
			onconfirm();
		}
	}

	function handleCancel() {
		if (!loading) {
			oncancel();
		}
	}
</script>

<Modal {title} {open} onclose={handleCancel} showClose={false}>
	{#snippet children()}
		<p class={styles.message}>{message}</p>
	{/snippet}

	{#snippet footer()}
		<div class={styles.footer}>
			<Button variant="secondary" onclick={handleCancel} disabled={loading}>
				{cancelLabel}
			</Button>
			<Button
				variant={variant === 'warning' ? 'primary' : variant}
				onclick={handleConfirm}
				{loading}
			>
				{confirmLabel}
			</Button>
		</div>
	{/snippet}
</Modal>
