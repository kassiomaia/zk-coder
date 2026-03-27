<script lang="ts">
    /**
     * FileDiffModal Component
     * Modal for viewing file diffs after agent completion.
     * Shows the file path, diff content, or current file content with syntax highlighting.
     */
    import Modal from '$lib/components/shared/Modal.svelte';
    import Button from '$lib/components/shared/Button.svelte';
    import DiffViewer from './DiffViewer.svelte';
    import {FilePlus, FileText, FileX, X} from 'lucide-svelte';
    import {type FileContentResponse, getFileContent} from '$lib/api/file';

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

<Modal open={open} onclose={onclose} size="full" showClose={false} contentClass="no-padding-body">
	<div class="file-diff-modal">
		<header class="modal-header-alt">
			<div class="header-left">
				<div class="file-icon-wrapper" class:is-new={fileData?.is_new} class:is-deleted={!fileData?.exists}>
					{#if fileData?.is_new || fileData?.current_content && !fileData?.original_content}
						<FilePlus size={16} strokeWidth={2.5} />
					{:else if !fileData?.exists}
						<FileX size={16} strokeWidth={2.5} />
					{:else}
						<FileText size={16} strokeWidth={2} />
					{/if}
				</div>
				<div class="file-info">
					<div class="file-name-row">
						<h3 class="file-name">{fileName}</h3>
						{#if fileData?.is_new}
							<span class="status-tag new">New</span>
						{:else if !fileData?.exists}
							<span class="status-tag deleted">Deleted</span>
						{:else}
							<span class="status-tag modified">Modified</span>
						{/if}
					</div>
					<p class="file-path">{filePath}</p>
				</div>
			</div>

			<div class="header-actions">
				<Button variant="ghost" size="sm" onclick={onclose} aria-label="Close modal">
					<X size={18} />
				</Button>
			</div>
		</header>

		<div class="modal-body">
			{#if diffToShow}
				<div class="diff-container-fixed">
					<DiffViewer diff={diffToShow} maxHeight={maxHeight} view={view} />
				</div>
			{:else if isLoading}
				<div class="loading">
					<div class="spinner"></div>
					<p>Analyzing changes...</p>
				</div>
			{:else if error}
				<div class="error-state">
					<p>{error}</p>
				</div>
			{:else if fileData?.current_content}
				<div class="file-content">
					<div class="content-header">
						<span class="content-label">
							{#if fileData.is_new}
								New File Content
							{:else}
								Final Result
							{/if}
						</span>
					</div>
					<pre class="file-content-pre"><code>{fileData.current_content}</code></pre>
				</div>
			{:else}
				<div class="error-state">
					<p>File not found or cannot be read.</p>
				</div>
			{/if}
		</div>
	</div>
</Modal>

<style>
	:global(.no-padding-body) {
		padding: 0 !important;
		overflow: hidden !important;
		display: flex !important;
		flex-direction: column !important;
	}

	.file-diff-modal {
		display: flex;
		flex-direction: column;
		height: 100%;
		flex: 1;
		min-height: 0;
		background: var(--color-bg-primary);
		overflow: hidden;
	}

	.modal-header-alt {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-6);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-primary);
		flex-shrink: 0;
		z-index: 10;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-width: 0;
	}

	.file-icon-wrapper {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		background: var(--color-neutral-800);
		border-radius: var(--radius-md);
		color: var(--color-text-muted);
	}

	.file-icon-wrapper.is-new {
		color: var(--color-success);
		background: var(--color-success-light);
	}

	.file-icon-wrapper.is-deleted {
		color: var(--color-error);
		background: var(--color-error-light);
	}

	.file-info {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}

	.file-name-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.file-name {
		font-size: var(--text-base);
		font-weight: 700;
		color: var(--color-text-primary);
		margin: 0;
		line-height: 1.2;
	}

	.status-tag {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 2px 6px;
		border-radius: 4px;
	}

	.status-tag.new { color: var(--color-success); background: var(--color-success-subtle); }
	.status-tag.modified { color: var(--color-warning); background: var(--color-warning-subtle); }
	.status-tag.deleted { color: var(--color-error); background: var(--color-error-subtle); }

	.file-path {
		font-size: 11px;
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		margin: 0;
		opacity: 0.7;
	}

	.modal-body {
		flex: 1;
		min-height: 0;
		background: var(--color-bg-primary);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.diff-container-fixed {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.file-content {
		display: flex;
		flex-direction: column;
		height: 100%;
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}

	.content-header {
		padding: var(--space-2) var(--space-4);
		background: var(--color-bg-secondary);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.content-label {
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		color: var(--color-text-muted);
		letter-spacing: 0.1em;
	}

	.file-content-pre {
		margin: 0;
		padding: var(--space-6);
		background: var(--color-bg-primary);
		overflow: auto;
		font-family: var(--font-mono);
		font-size: 13px;
		line-height: 1.6;
	}

	.error-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: var(--color-error);
		font-size: var(--text-sm);
		font-style: italic;
	}

	.loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-3);
		height: 100%;
		color: var(--color-text-muted);
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--color-border-subtle);
		border-top-color: var(--color-primary-500);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}
</style>
