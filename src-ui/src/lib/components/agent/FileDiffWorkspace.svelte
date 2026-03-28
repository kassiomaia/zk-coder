<script lang="ts">
	/**
	 * FileDiffWorkspace Component
	 * Standalone Tauri window for code review.
	 * Uses the shared WindowHeader for consistent chrome across all Tauri windows.
	 */
	import DiffViewer from './DiffViewer.svelte';
	import WindowHeader from '$lib/components/shared/WindowHeader.svelte';
	import {
		Columns,
		Rows,
		Code2,
		GitCompare
	} from 'lucide-svelte';
	import { type FileContentResponse, getFileContent } from '$lib/api/file';

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

	let viewMode = $state<'diff' | 'full'>('diff');
	let diffStyle = $state<'side-by-side' | 'inline'>('side-by-side');
	let showMinimap = $state(true);

	let fileName = $derived(filePath.split('/').pop() ?? filePath);

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

	function handleContextMenu(e: MouseEvent) {
		e.preventDefault();
	}

	function handleKeyDown(e: KeyboardEvent) {
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
	<title>Code Viewer - {fileName}</title>
</svelte:head>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="workspace" oncontextmenu={handleContextMenu} onkeydown={handleKeyDown}>
	<WindowHeader title="Code Viewer" subtitle={filePath} />

	<!-- Toolbar - view controls -->
	<div class="toolbar">
		<div class="toolbar-right">
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
	</div>

	<!-- Editor area -->
	<main class="editor-area">
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
			<div class="editor-container">
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

<style>
	.workspace {
		display: flex;
		flex-direction: column;
		height: 100vh;
		width: 100vw;
		background: var(--color-bg-primary);
		overflow: hidden;
		box-sizing: border-box;
		border-radius: var(--radius-xl);
		border: 1px solid var(--color-border-primary);
		box-shadow: var(--shadow-2xl);
	}

	/* Toolbar - view controls */
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-4);
		background: var(--color-bg-primary);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}

	.toolbar-right {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-shrink: 0;
	}

	.segmented-control {
		display: flex;
		background: var(--color-bg-tertiary);
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
		background: var(--color-surface);
		color: var(--color-text-primary);
		box-shadow: var(--shadow-sm);
	}

	.vertical-divider {
		width: 1px;
		height: 16px;
		background: var(--color-border-subtle);
		margin: 0 var(--space-0-5);
	}

	/* Editor area */
	.editor-area {
		flex: 1;
		min-height: 0;
		position: relative;
		background: var(--color-bg-secondary);
		padding: var(--space-3);
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
		background: var(--color-editor-bg);
		box-shadow: var(--shadow-sm);
		position: relative;
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
</style>
