<script lang="ts">
    /**
     * WorkspaceSelector Component
     * Displays the current workspace path and allows selecting a new one
     * via the native OS folder picker dialog.
     */
    import {FolderClosed, FolderOpen} from 'lucide-svelte';
    import {open} from '@tauri-apps/plugin-dialog';

    interface Props {
		workspacePath?: string;
		onSelect: (path: string) => void;
		disabled?: boolean;
	}

	let { workspacePath, onSelect, disabled = false }: Props = $props();

	let selecting = $state(false);

	async function openFolderPicker() {
		if (selecting || disabled) return;
		selecting = true;
		try {
			const result = await open({
				directory: true,
				multiple: false,
				title: 'Select Workspace Folder'
			});
			if (typeof result === 'string') {
				onSelect(result);
			}
		} catch (error) {
			console.error('Failed to open folder picker:', error);
		} finally {
			selecting = false;
		}
	}

	function getDisplayPath(path?: string): string {
		if (!path) return 'No workspace selected';
		const parts = path.replace(/\\/g, '/').split('/').filter(Boolean);
		if (parts.length === 0) return path;
		// If it's a deep path, show only the last segment as the "Project Name"
		return parts[parts.length - 1];
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
		<FolderOpen size={12} strokeWidth={2.5} />
	{:else}
		<FolderClosed size={12} strokeWidth={2.5} />
	{/if}
	<span class="workspace-label">Working in</span>
	<span class="workspace-path">{getDisplayPath(workspacePath)}</span>
</button>

<style>
	.workspace-selector {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: var(--space-0-5) 0;
		font-size: 11px;
		color: var(--color-text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: color var(--transition-fast);
		overflow: hidden;
		max-width: 100%;
	}

	.workspace-selector:hover:not(.disabled) {
		color: var(--color-text-primary);
	}

	.workspace-selector:focus-visible {
		outline: none;
		color: var(--color-primary-400);
	}

	.workspace-selector.disabled {
		cursor: default;
		opacity: var(--opacity-disabled);
	}

	.workspace-selector.has-path {
		color: var(--color-text-muted);
	}

	.workspace-label {
		opacity: 0.5;
		margin-right: var(--space-1);
	}

	.workspace-path {
		color: var(--color-text-secondary);
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		border-bottom: 1px dashed var(--color-border-subtle);
	}
</style>
