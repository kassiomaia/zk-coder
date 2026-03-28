<script lang="ts">
	/**
	 * FileDiffWorkspace Component
	 * Enterprise-grade code review workspace with Monaco Editor.
	 * Designed for use as a standalone chromeless Tauri window.
	 */
	import DiffViewer from './DiffViewer.svelte';
	import { 
		FileText, 
		FilePlus, 
		FileX, 
		X, 
		Copy, 
		Columns, 
		Rows, 
		Code2,
		GitCompare,
		Minus,
		Square,
		Maximize2
	} from 'lucide-svelte';
	import { type FileContentResponse, getFileContent } from '$lib/api/file';
	
	import Check from 'lucide-svelte/icons/check';
	import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

	interface Props {
		filePath: string;
		workspaceRoot: string;
		sessionId: string;
		existingDiff?: string;
	}

	let {
		filePath,
		workspaceRoot,
		sessionId,
		existingDiff,
	}: Props = $props();

	let fileData = $state<FileContentResponse | null>(null);
	let isLoading = $state(false);
	let error = $state<string | null>(null);
	let isCopied = $state(false);
	let isMaximized = $state(false);
	
	let viewMode = $state<'diff' | 'full'>('diff');
	let diffStyle = $state<'side-by-side' | 'inline'>('side-by-side');
	let showMinimap = $state(true);

	const currentWindow = getCurrentWebviewWindow();

	let fileName = $derived(filePath.split('/').pop() ?? filePath);
	let fileDir = $derived(filePath.split('/').slice(0, -1).join('/') || './');
	let status = $derived(
		!fileData ? 'loading' :
		fileData.is_new ? 'new' : 
		(!fileData.exists ? 'deleted' : 'modified')
	);

	$effect(() => {
		if (workspaceRoot && sessionId && filePath) {
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

	$effect(() => {
		currentWindow.onResized(async () => {
			isMaximized = await currentWindow.isMaximized();
		});
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

	async function handleMinimize() {
		await currentWindow.minimize();
	}

	async function handleMaximize() {
		const isMax = await currentWindow.isMaximized();
		if (isMax) {
			await currentWindow.unmaximize();
		} else {
			await currentWindow.maximize();
		}
		// State will be updated via the onResized effect
	}

	async function handleClose() {
		await currentWindow.close();
	}

	async function handleHeaderDblClick(e: MouseEvent) {
		// Only maximize if clicking on the drag region itself, not on buttons
		if ((e.target as HTMLElement).closest('button')) return;
		await handleMaximize();
	}

	async function handleHeaderMouseDown(e: MouseEvent) {
		// Only left-click (button 0)
		if (e.button !== 0) return;
		// Prevent dragging when clicking interactive elements
		if ((e.target as HTMLElement).closest('button, .segmented-control, .action-group')) return;
		
		await currentWindow.startDragging();
	}

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
	}

	function handleKeyDown(e: KeyboardEvent) {
		// Disable DevTools shortcuts (F12, Ctrl+Shift+I, Cmd+Option+I)
		if (
			e.key === 'F12' ||
			((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'I') ||
			((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'J') ||
			((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'C') ||
			(e.metaKey && e.altKey && e.key === 'i')
		) {
			e.preventDefault();
		}
	}
</script>

<svelte:head>
	<title>{fileName} - File Diff</title>
</svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="review-workspace" oncontextmenu={handleContextMenu} onkeydown={handleKeyDown}>
	<div class="workspace-card shadow-lg">
		<header 
			class="workspace-header" 
			onmousedown={handleHeaderMouseDown} 
			ondblclick={handleHeaderDblClick}
		>
		<div class="header-content">
			<div class="header-zone left">
				<div 
					class="file-icon-badge" 
					class:new={status === 'new'} 
					class:deleted={status === 'deleted'}
					class:modified={status === 'modified'}
					class:loading={status === 'loading'}
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
					{#if status !== 'loading'}
						<span class="status-pill {status}">{status}</span>
					{/if}
				</div>
			</div>

			<div class="header-zone center"></div>

			<div class="header-zone right">
				<div class="action-group">
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

				<div class="window-controls">
					<button 
						class="window-btn minimize" 
						onclick={handleMinimize}
						title="Minimize"
						type="button"
					>
						<Minus size={14} />
					</button>
					<button 
						class="window-btn maximize" 
						onclick={handleMaximize}
						title={isMaximized ? 'Restore' : 'Maximize'}
						type="button"
					>
						{#if isMaximized}
							<Square size={12} />
						{:else}
							<Maximize2 size={14} />
						{/if}
					</button>
					<button 
						class="window-btn close" 
						onclick={handleClose}
						title="Close"
						type="button"
					>
						<X size={14} />
					</button>
				</div>
			</div>
		</div>
		</header>

		<main class="workspace-body">
			{#if isLoading}
				<div class="state-overlay">
					<div class="spinner"></div>
					<p class="loading-text">Analyzing workspace changes...</p>
				</div>
			{:else if error}
				<div class="state-overlay error">
					<p>{error}</p>
				</div>
			{:else if fileData}
				<div 
					class="editor-container" 
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
</div>

<style>
	.review-workspace {
		display: flex;
		flex-direction: column;
		height: 100vh;
		width: 100vw;
		background: var(--color-bg-primary);
		overflow: hidden;
		box-sizing: border-box;
	}

	.workspace-card {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		background: var(--color-bg-primary);
		overflow: hidden;
	}

	.workspace-header {
		height: 48px;
		min-height: 48px;
		display: flex;
		align-items: center;
		padding: 0 var(--space-4);
		background: var(--color-bg-primary);
		border-bottom: 1px solid var(--color-border-subtle);
		border-top: 3px solid var(--color-primary-500);
		z-index: var(--z-above);
		flex-shrink: 0;
		user-select: none;
		-webkit-user-select: none;
		pointer-events: auto;
	}

	/* Target children to allow drag, but keep header interactive for dblclick */
	.header-content {
		display: flex;
		align-items: center;
		width: 100%;
		height: 100%;
		pointer-events: none; /* Let clicks pass to header */
	}

	.header-zone, .window-controls {
		pointer-events: auto; /* Re-enable for interactive elements */
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

	.file-icon-badge.loading {
		animation: pulse 1.5s ease-in-out infinite;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
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

	.window-controls {
		display: flex;
		align-items: center;
		gap: 0;
		margin-left: var(--space-2);
		position: relative;
		z-index: 1000;
		-webkit-app-region: no-drag;
	}

	.window-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 32px;
		border: none;
		background: transparent;
		color: var(--color-text-muted);
		cursor: pointer;
		transition: all var(--transition-fast);
		pointer-events: auto;
		-webkit-app-region: no-drag;
	}

	.window-btn:hover {
		background: var(--color-surface-hover);
		color: var(--color-text-primary);
	}

	.window-btn.close:hover {
		background: var(--color-error);
		color: white;
	}

	.workspace-body {
		flex: 1;
		min-height: 0;
		position: relative;
		background: var(--depth-bg-1);
		padding: var(--space-4);
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
		box-shadow: var(--shadow-lg);
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
