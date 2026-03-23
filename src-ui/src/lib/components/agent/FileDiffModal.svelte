<script lang="ts">
	/**
	 * FileDiffModal Component
	 * Modal for viewing file diffs after agent completion.
	 * Shows the file path and diff content with syntax highlighting.
	 */
	import Modal from '$lib/components/shared/Modal.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import DiffViewer from './DiffViewer.svelte';
	import { FileText } from 'lucide-svelte';

	interface Props {
		/** Whether the modal is open */
		open: boolean;
		/** Close handler */
		onclose: () => void;
		/** Path of the file being viewed */
		filePath: string;
		/** Unified diff content */
		diff: string;
		/** Maximum height of the diff viewer */
		maxHeight?: number;
		/** Initial view mode */
		view?: 'unified' | 'side-by-side';
	}

	let { open, onclose, filePath, diff, maxHeight = 600, view = 'unified' }: Props = $props();

	// Extract filename from path for display
	let fileName = $derived(filePath.split('/').pop() ?? filePath);
</script>

<Modal open={open} onclose={onclose} size="full" title={undefined}>
	<div class="file-diff-modal">
		<header class="modal-header-alt">
			<div class="header-content">
				<FileText size={18} />
				<div class="file-info">
					<h3 class="file-name">{fileName}</h3>
					<p class="file-path">{filePath}</p>
				</div>
			</div>
		</header>

		<div class="modal-body">
			{#if diff}
				<DiffViewer diff={diff} maxHeight={maxHeight} view={view} />
			{:else}
				<div class="no-diff">
					<p>No diff available for this file.</p>
				</div>
			{/if}
		</div>

		<footer class="modal-footer">
			<Button variant="primary" onclick={onclose}>Close</Button>
		</footer>
	</div>
</Modal>

<style>
	.file-diff-modal {
		display: flex;
		flex-direction: column;
		height: 100%;
		max-height: 92vh;
	}

	/* Custom header with file info */
	.modal-header-alt {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-4) var(--space-6);
		border-bottom: 1px solid var(--color-border-subtle);
		background: var(--color-surface);
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}

	.header-content :global(svg) {
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.file-info {
		display: flex;
		flex-direction: column;
		gap: var(--space-0-5);
		min-width: 0;
	}

	.file-name {
		font-size: var(--text-base);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-path {
		font-size: var(--text-xs);
		font-family: var(--font-mono);
		color: var(--color-text-muted);
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* Body with diff viewer */
	.modal-body {
		flex: 1;
		min-height: 0;
		overflow: hidden;
		padding: var(--space-4) var(--space-6);
		display: flex;
		flex-direction: column;
	}

	.no-diff {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	/* Footer with close button */
	.modal-footer {
		display: flex;
		justify-content: flex-end;
		padding: var(--space-3) var(--space-6);
		border-top: 1px solid var(--color-border-subtle);
		background: var(--color-surface);
		flex-shrink: 0;
	}
</style>
