<script lang="ts">
	/**
	 * FileDiffModal Component
	 * Modal for viewing file diffs after agent completion.
	 * Shows the file path, diff content, or current file content with syntax highlighting.
	 */
	import Modal from '$lib/components/shared/Modal.svelte';
	import Button from '$lib/components/shared/Button.svelte';
	import DiffViewer from './DiffViewer.svelte';
	import { FileText, FilePlus, FileX } from 'lucide-svelte';
	import { getFileContent, type FileContentResponse } from '$lib/api/file';

	interface Props {
		/** Whether the modal is open */
		open: boolean;
		/** Close handler */
		onclose: () => void;
		/** Path of the file being viewed */
		filePath: string;
		/** Workspace root directory */
		workspaceRoot: string;
		/** Agent session ID for rollback store access */
		sessionId: string;
		/** Pre-fetched diff from filesTouched (if available) */
		existingDiff?: string;
		/** Maximum height of the diff viewer */
		maxHeight?: number;
		/** Initial view mode */
		view?: 'unified' | 'side-by-side';
	}

	let {
		open,
		onclose,
		filePath,
		workspaceRoot,
		sessionId,
		existingDiff,
		maxHeight = 600,
		view = 'unified'
	}: Props = $props();

	// State for fetched file content
	let fileData = $state<FileContentResponse | null>(null);
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	// Extract filename from path for display
	let fileName = $derived(filePath.split('/').pop() ?? filePath);

	// Determine which diff to show
	let diffToShow = $derived(existingDiff || fileData?.diff);

	// Fetch file content when modal opens and no existing diff
	$effect(() => {
		if (open && !existingDiff && workspaceRoot && sessionId) {
			isLoading = true;
			error = null;
			getFileContent(workspaceRoot, filePath, sessionId)
				.then((data) => {
					fileData = data;
				})
				.catch((e) => {
					console.error('Failed to load file content:', e);
					error = e instanceof Error ? e.message : 'Failed to load file content';
				})
				.finally(() => {
					isLoading = false;
				});
		}
	});

	// Reset state when modal closes
	$effect(() => {
		if (!open) {
			fileData = null;
			error = null;
		}
	});
</script>

<Modal open={open} onclose={onclose} size="full" title={undefined}>
	<div class="file-diff-modal">
		<header class="modal-header-alt">
			<div class="header-content">
				{#if fileData?.is_new || fileData?.current_content && !fileData?.original_content}
					<FilePlus size={18} class="icon-new" />
				{:else if !fileData?.exists}
					<FileX size={18} class="icon-deleted" />
				{:else}
					<FileText size={18} />
				{/if}
				<div class="file-info">
					<h3 class="file-name">{fileName}</h3>
					<p class="file-path">{filePath}</p>
				</div>
			</div>
		</header>

		<div class="modal-body">
			{#if diffToShow}
				<DiffViewer diff={diffToShow} maxHeight={maxHeight} view={view} />
			{:else if isLoading}
				<div class="loading">
					<p>Loading file content...</p>
				</div>
			{:else if error}
				<div class="no-content">
					<p>{error}</p>
				</div>
			{:else if fileData?.current_content}
				<div class="file-content">
					<div class="content-header">
						<span class="content-label">
							{#if fileData.is_new}
								New file
							{:else}
								Current content
							{/if}
						</span>
					</div>
					<pre class="file-content-pre"><code>{fileData.current_content}</code></pre>
				</div>
			{:else}
				<div class="no-content">
					<p>File not found or cannot be read.</p>
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

	.header-content :global(.icon-new) {
		color: var(--color-success);
	}

	.header-content :global(.icon-deleted) {
		color: var(--color-error);
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

	/* Loading state */
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	/* No content state */
	.no-content {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-text-muted);
		font-size: var(--text-sm);
	}

	/* File content display */
	.file-content {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}

	.content-header {
		padding: var(--space-2) var(--space-3);
		background: var(--color-surface);
		border-radius: var(--radius-sm) var(--radius-sm) 0 0;
		border: 1px solid var(--color-border-subtle);
		border-bottom: none;
	}

	.content-label {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: var(--tracking-wide);
	}

	.file-content-pre {
		flex: 1;
		margin: 0;
		padding: var(--space-4);
		background: var(--color-bg-secondary);
		border: 1px solid var(--color-border-subtle);
		border-radius: 0 0 var(--radius-sm) var(--radius-sm);
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		line-height: 1.6;
		color: var(--color-text-primary);
	}

	.file-content-pre code {
		font-family: inherit;
		font-size: inherit;
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
