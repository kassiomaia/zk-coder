<script lang="ts">
	/**
	 * WorkspaceSelector Component
	 * Displays the current workspace path and allows selecting a new one
	 * via the native OS folder picker dialog.
	 */
	import { FolderOpen, FolderClosed } from 'lucide-svelte';

	interface Props {
		workspacePath?: string;
		onSelect: (path: string) => void;
		disabled?: boolean;
	}

	let { workspacePath, onSelect, disabled = false }: Props = $props();

	let selecting = $state(false);

	function getDisplayPath(path?: string): string {
		if (!path) return 'No workspace selected';
		// Show the last 2 segments for brevity
		const parts = path.replace(/\\/g, '/').split('/').filter(Boolean);
		if (parts.length <= 2) return path;
		return '.../' + parts.slice(-2).join('/');
	}

	async function openFolderPicker() {
		if (disabled || selecting) return;
		selecting = true;
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const selected = await open({
				directory: true,
				multiple: false,
				title: 'Select Workspace Directory',
				defaultPath: workspacePath || undefined
			});
			if (selected && typeof selected === 'string') {
				onSelect(selected);
			}
		} catch {
			// Dialog cancelled or Tauri not available
		} finally {
			selecting = false;
		}
	}
</script>

<button
	class="workspace-selector"
	class:disabled
	class:has-path={!!workspacePath}
	onclick={openFolderPicker}
	type="button"
	title={workspacePath || 'Select workspace directory'}
	{disabled}
>
	{#if workspacePath}
		<FolderOpen size={13} />
	{:else}
		<FolderClosed size={13} />
	{/if}
	<span class="workspace-path">{getDisplayPath(workspacePath)}</span>
</button>

<style>
	.workspace-selector {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		width: 100%;
		padding: var(--space-1-5) var(--space-4);
		font-size: var(--text-2xs);
		font-family: var(--font-mono);
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		border-bottom: 1px solid var(--color-border-subtle);
		cursor: pointer;
		text-align: left;
		transition:
			background var(--transition-fast),
			color var(--transition-fast);
		overflow: hidden;
	}

	.workspace-selector:hover:not(.disabled) {
		background: var(--color-interactive-hover);
		color: var(--color-text-secondary);
	}

	.workspace-selector:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.workspace-selector.disabled {
		cursor: default;
		opacity: var(--opacity-disabled);
	}

	.workspace-selector.has-path {
		color: var(--color-text-secondary);
	}

	.workspace-path {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		flex: 1;
	}
</style>
