<script lang="ts">
	/**
	 * WorkspaceSelector Component
	 * Displays the current workspace path and allows selecting a new one
	 * via the native OS folder picker dialog.
	 */
	import { FolderClosed, FolderOpen } from 'lucide-svelte';
	import { open } from '@tauri-apps/plugin-dialog';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('WorkspaceSelector');

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
			log.error('Failed to open folder picker:', error);
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
		gap: var(--space-1);
		padding: 0;
		font-size: var(--text-2xs);
		color: var(--color-text-tertiary);
		background: transparent;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: all var(--transition-fast);
		overflow: hidden;
		max-width: 100%;
		height: 100%;
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
		color: var(--color-text-tertiary);
	}

	.workspace-label {
		display: none;
	}

	.workspace-path {
		color: inherit;
		font-weight: var(--font-medium);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		border-bottom: 1px dashed transparent;
	}

	.workspace-selector:hover .workspace-path {
		border-color: currentColor;
	}
</style>
