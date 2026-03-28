<script lang="ts">
	/**
	 * FileDiffModal Component
	 * Enterprise-grade code review workspace with Monaco Editor.
	 * Features structured control bar, precise layout, and high-quality interaction.
	 */
	import Modal from '$lib/components/shared/Modal.svelte';
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import DiffViewer from './DiffViewer.svelte';
	import { 
		FileText, 
		FilePlus, 
		FileX, 
		X, 
		Copy, 
		Maximize2, 
		Columns, 
		Rows, 
		Code2,
		GitCompare
	} from 'lucide-svelte';
	import { type FileContentResponse, getFileContent } from '$lib/api/file';
	import { fade, fly } from 'svelte/transition';
	import { quintOut } from 'svelte/easing';
	import Check from 'lucide-svelte/icons/check';

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
		/** Pre-fetched diff (fallback) */
		existingDiff?: string;
	}

	let {
		open,
		onclose,
		filePath,
		workspaceRoot,
		sessionId,
	}: Props = $props();

	// State for fetched file content
	let fileData = $state<FileContentResponse | null>(null);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let isCopied = $state(false);
	
	// Review View Options
	let viewMode = $state<'diff' | 'full'>('diff');
	let diffStyle = $state<'side-by-side' | 'inline'>('side-by-side');
	let showMinimap = $state(true);

	// Derived UI state
	let fileName = $derived(filePath.split('/').pop() ?? filePath);
	let fileDir = $derived(filePath.split('/').slice(0, -1).join('/') || './');
	let status = $derived(
		fileData?.is_new ? 'new' : 
		(!fileData?.exists ? 'deleted' : 'modified')
	);

	// Fetch file content when modal opens
	$effect(() => {
		if (open && workspaceRoot && sessionId) {
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
			viewMode = 'diff';
			diffStyle = 'side-by-side';
			isCopied = false;
		}
	});

	async function copyToClipboard(text: string) {
		try {
			await navigator.clipboard.writeText(text);
			isCopied = true;
			setTimeout(() => {
				isCopied = false;
			}, 2000);
		} catch (err) {
			console.error('Failed to copy text: ', err);
		}
	}
</script>

<Modal 
	open={open} 
	onclose={onclose} 
	size="full" 
	showClose={false} 
	contentClass="no-padding-body"
>
	<div 
		class="review-workspace" 
		in:fly={{ y: 20, duration: 400, easing: quintOut }}
	>
		<header class="workspace-header">
			<div class="header-content">
				<!-- Left Zone: File Identity -->
				<div class="header-zone left">
					<div 
						class="file-icon-badge" 
						class:new={status === 'new'} 
						class:deleted={status === 'deleted'}
						class:modified={status === 'modified'}
					>
						{#if status === 'new'}
							<FilePlus size={14} />
						{:else if status === 'deleted'}
							<FileX size={14} />
						{:else}
							<FileText size={14} />
						{/if}
					</div>
					
					<div class="file-identity">
						<h3 class="file-name">{fileName}</h3>
						<span class="file-path">{fileDir}/</span>
						<span class="status-pill {status}">{status}</span>
					</div>
				</div>

				<!-- Center Zone: Spacer/Flexible Area -->
				<div class="header-zone center"></div>

				<!-- Right Zone: Actions -->
				<div class="header-zone right">
					<div class="action-group">
						<!-- View Mode Toggles -->
						<div class="segmented-control">
							<button 
								class="segment-item" 
								class:active={viewMode === 'diff'} 
								onclick={() => viewMode = 'diff'}
								title="Show changes"
							>
								<GitCompare size={14} />
								<span>Changes</span>
							</button>
							<button 
								class="segment-item" 
								class:active={viewMode === 'full'} 
								onclick={() => viewMode = 'full'}
								title="Show final result"
							>
								<Code2 size={14} />
								<span>Final Result</span>
							</button>
						</div>

						{#if viewMode === 'diff'}
							<div class="vertical-divider"></div>
							<div class="segmented-control">
								<button 
									class="segment-item icon-only" 
									class:active={diffStyle === 'side-by-side'} 
									onclick={() => diffStyle = 'side-by-side'}
									title="Side by Side"
								>
									<Columns size={14} />
								</button>
								<button 
									class="segment-item icon-only" 
									class:active={diffStyle === 'inline'} 
									onclick={() => diffStyle = 'inline'}
									title="Unified View"
								>
									<Rows size={14} />
								</button>
							</div>
						{/if}
					</div>

					<div class="vertical-divider"></div>

					<div class="utility-group">
						<IconButton 
							icon={closeIcon} 
							size="sm" 
							variant="ghost" 
							label="Close" 
							onclick={onclose}
							class="close-action"
						/>
					</div>
				</div>
			</div>
		</header>

		<main class="workspace-body">
			{#if isLoading}
				<div class="state-overlay" in:fade={{ duration: 150 }}>
					<div class="spinner"></div>
					<p class="loading-text">Analyzing workspace changes...</p>
				</div>
			{:else if error}
				<div class="state-overlay error" in:fade={{ duration: 150 }}>
					<p>{error}</p>
				</div>
			{:else if fileData}
				<div 
					class="editor-container" 
					in:fade={{ duration: 300, delay: 100 }}
				>
					<DiffViewer 
						originalContent={fileData.original_content || ''}
						modifiedContent={fileData.current_content || ''}
						filePath={filePath}
						mode={viewMode}
						diffStyle={diffStyle}
						minimap={showMinimap}
					/>
				</div>
			{:else}
				<div class="state-overlay">
					<p>File content unavailable</p>
				</div>
			{/if}
		</main>
	</div>
</Modal>

{#snippet copyIcon()}
	<Copy size={16} />
{/snippet}

{#snippet copySuccessIcon()}
	<Check size={16} class="text-success" />
{/snippet}

{#snippet maximizeIcon()}
	<Maximize2 size={16} />
{/snippet}

{#snippet closeIcon()}
	<X size={18} />
{/snippet}

<style>
	.review-workspace {
		display: flex;
		flex-direction: column;
		height: 94vh;
		max-height: 94vh;
		min-height: 700px;
		background: var(--color-bg-primary);
		overflow: hidden;
		border-radius: var(--radius-xl);
		margin: 0 auto;
		width: 100%;
	}

	.workspace-header {
		height: 48px;
		min-height: 48px;
		display: flex;
		align-items: center;
		padding: 0 var(--space-4);
		background: var(--color-bg-primary);
		border-bottom: 1px solid var(--color-border-subtle);
		z-index: var(--z-above);
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		align-items: center;
		width: 100%;
		height: 100%;
	}

	.header-zone {
		display: flex;
		align-items: center;
		min-width: 0;
	}

	.header-zone.left {
		flex: 1;
		gap: var(--space-3);
	}

	.header-zone.center {
		display: none;
	}

	.header-zone.right {
		flex-shrink: 0;
		gap: var(--space-2);
	}

	.file-identity {
		display: flex;
		align-items: center;
		min-width: 0;
		gap: var(--space-2);
	}

	.file-name {
		font-size: var(--text-sm);
		font-weight: var(--font-semibold);
		color: var(--color-text-primary);
		margin: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		line-height: 1;
	}

	.file-path {
		font-size: var(--text-xs);
		color: var(--color-text-muted);
		font-family: var(--font-mono);
		opacity: 0.7;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		margin-top: 1px;
	}

	.file-icon-badge {
		width: 28px;
		height: 28px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		background: var(--color-surface-hover);
		color: var(--color-text-secondary);
		border: 1px solid var(--color-border-subtle);
		transition: all var(--transition-normal);
		flex-shrink: 0;
	}

	.file-icon-badge.new {
		background: var(--color-success-subtle);
		color: var(--color-success);
		border-color: var(--color-success-border);
	}

	.file-icon-badge.deleted {
		background: var(--color-error-light);
		color: var(--color-error);
		border-color: var(--color-error-border);
		opacity: 0.9;
	}

	.file-icon-badge.modified {
		background: var(--color-warning-light);
		color: var(--color-warning);
		border-color: var(--color-warning-border);
	}

	.status-pill {
		font-size: 10px;
		padding: 1px 6px;
		border-radius: var(--radius-sm);
		font-weight: var(--font-bold);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		flex-shrink: 0;
		border: 1px solid transparent;
		line-height: 1.4;
	}

	.status-pill.new {
		background: var(--color-success-subtle);
		color: var(--color-success);
		border-color: var(--color-success-border);
	}

	.status-pill.deleted {
		background: var(--color-error-light);
		color: var(--color-error);
		border-color: var(--color-error-border);
	}

	.status-pill.modified {
		background: var(--color-warning-light);
		color: var(--color-warning);
		border-color: var(--color-warning-border);
	}

	.action-group {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.segmented-control {
		display: flex;
		background: var(--color-surface-hover);
		padding: 2px;
		border-radius: var(--radius-md);
		border: 1px solid var(--color-border-subtle);
		gap: 1px;
	}

	.segment-item {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		padding: var(--space-1) var(--space-2-5);
		border-radius: var(--radius-sm);
		font-size: 11px;
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		transition: all var(--transition-fast);
		border: none;
		background: transparent;
		cursor: pointer;
		white-space: nowrap;
		line-height: 1;
	}

	.segment-item.icon-only {
		padding: var(--space-1) var(--space-1-5);
	}

	.segment-item:hover:not(.active) {
		color: var(--color-text-primary);
		background: var(--color-interactive-hover);
	}

	.segment-item.active {
		background: var(--color-bg-primary);
		color: var(--color-primary-500);
		box-shadow: var(--shadow-sm);
	}

	.vertical-divider {
		width: 1px;
		height: 16px;
		background: var(--color-border-subtle);
		margin: 0 var(--space-0-5);
	}

	.vertical-divider-soft {
		width: 1px;
		height: 14px;
		background: var(--color-border-subtle);
		margin: 0 var(--space-0-5);
		opacity: 0.5;
	}

	.utility-group {
		display: flex;
		align-items: center;
		gap: 0;
	}

	:global(.utility-group .close-action) {
		color: var(--color-text-muted);
		transition: color var(--transition-fast);
	}

	:global(.utility-group .close-action:hover) {
		color: var(--color-error);
		background: var(--color-error-light) !important;
	}

	.workspace-body {
		flex: 1;
		min-height: 0;
		position: relative;
		background: var(--color-bg-secondary);
		padding: var(--layout-padding);
		overflow: hidden;
		display: flex;
		flex-direction: column;
	}

	.editor-container {
		flex: 1;
		min-height: 0;
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid var(--color-border-subtle);
		background: var(--color-bg-primary);
		box-shadow: var(--shadow-xl);
	}

	.state-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-4);
		color: var(--color-text-secondary);
		background: var(--color-bg-secondary);
		z-index: var(--z-above);
	}

	.loading-text {
		font-size: var(--text-sm);
		font-weight: var(--font-medium);
		color: var(--color-text-muted);
	}

	.spinner {
		width: 24px;
		height: 24px;
		border: 2px solid var(--color-primary-subtle);
		border-top-color: var(--color-primary-500);
		border-radius: var(--radius-full);
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.text-success {
		color: var(--color-success);
	}
</style>
