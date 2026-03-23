<script lang="ts">
	/**
	 * FileTree Component
	 * Displays files touched by the agent in a hierarchical tree.
	 */
	import {
		File,
		FileCode,
		FileText,
		Folder,
		FolderOpen,
		ChevronRight,
		ChevronDown
	} from 'lucide-svelte';
	import type { TouchedFile } from '$lib/types/agent';

	interface FileTreeNode {
		name: string;
		path: string;
		type: 'file' | 'directory';
		children: FileTreeNode[];
		file?: TouchedFile;
	}

	interface Props {
		files: TouchedFile[];
		onfileclick?: (path: string) => void;
	}

	let { files, onfileclick }: Props = $props();

	/** Track expanded directories */
	let expandedPaths = $state<Set<string>>(new Set());

	/** Build hierarchical tree from flat file list */
	let fileTree = $derived(buildFileTree(files));

	function buildFileTree(flatFiles: TouchedFile[]): FileTreeNode {
		const root: FileTreeNode = { name: '', path: '', type: 'directory', children: [] };

		for (const file of flatFiles) {
			const parts = file.path.split('/');
			let current = root;

			for (let i = 0; i < parts.length; i++) {
				const part = parts[i];
				const isFile = i === parts.length - 1;
				let child = current.children.find((c) => c.name === part);

				if (!child) {
					child = {
						name: part,
						path: parts.slice(0, i + 1).join('/'),
						type: isFile ? 'file' : 'directory',
						children: [],
						file: isFile ? file : undefined
					};
					current.children.push(child);
				}

				if (!isFile) {
					current = child;
				}
			}
		}

		return root;
	}

	function toggleExpanded(path: string) {
		const next = new Set(expandedPaths);
		if (next.has(path)) {
			next.delete(path);
		} else {
			next.add(path);
		}
		expandedPaths = next;
	}

	function getFileIcon(filePath: string) {
		const ext = filePath.split('.').pop()?.toLowerCase();
		switch (ext) {
			case 'ts':
			case 'js':
			case 'tsx':
			case 'jsx':
			case 'rs':
			case 'py':
			case 'go':
				return FileCode;
			case 'md':
			case 'txt':
			case 'toml':
			case 'yaml':
			case 'yml':
			case 'json':
				return FileText;
			default:
				return File;
		}
	}

	function getChangeIndicator(file: TouchedFile): { label: string; cssClass: string } | null {
		if (file.action === 'write' && file.modified) return { label: 'M', cssClass: 'modified' };
		if (file.action === 'write' && !file.modified) return { label: 'A', cssClass: 'added' };
		if (file.action === 'delete') return { label: 'D', cssClass: 'deleted' };
		return null;
	}
</script>

<div class="file-tree" role="tree" aria-label="File tree">
	{#each fileTree.children as node (node.path)}
		{#if node.type === 'directory'}
			{@const isExpanded = expandedPaths.has(node.path)}
			<button
				class="tree-node directory"
				onclick={() => toggleExpanded(node.path)}
				type="button"
				role="treeitem"
				aria-expanded={isExpanded}
			>
				{#if isExpanded}
					<ChevronDown size={12} />
					<FolderOpen size={14} />
				{:else}
					<ChevronRight size={12} />
					<Folder size={14} />
				{/if}
				<span>{node.name}/</span>
			</button>

			{#if isExpanded}
				{#each node.children as child (child.path)}
					{#if child.type === 'file' && child.file}
						{@const indicator = getChangeIndicator(child.file)}
						<button
							class="tree-node file"
							class:modified={child.file.action === 'write'}
							class:deleted={child.file.action === 'delete'}
							style="padding-left: 24px"
							onclick={() => onfileclick?.(child.path)}
							type="button"
							role="treeitem"
						>
							<svelte:component this={getFileIcon(child.path)} size={14} />
							<span>{child.name}</span>
							{#if indicator}
								<span class="change-indicator {indicator.cssClass}">{indicator.label}</span>
							{/if}
						</button>
					{/if}
				{/each}
			{/if}
		{:else if node.file}
			{@const indicator = getChangeIndicator(node.file)}
			<button
				class="tree-node file"
				class:modified={node.file.action === 'write'}
				class:deleted={node.file.action === 'delete'}
				onclick={() => onfileclick?.(node.path)}
				type="button"
				role="treeitem"
			>
				<svelte:component this={getFileIcon(node.path)} size={14} />
				<span>{node.name}</span>
				{#if indicator}
					<span class="change-indicator {indicator.cssClass}">{indicator.label}</span>
				{/if}
			</button>
		{/if}
	{:else}
		<div class="empty-state">
			<p>No files touched</p>
		</div>
	{/each}
</div>

<style>
	.file-tree {
		padding: var(--space-2) var(--space-4);
	}

	.tree-node {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
		width: 100%;
		padding: var(--space-1) var(--space-2);
		padding-left: var(--space-3);
		font-size: var(--text-xs);
		color: var(--color-text-secondary);
		border-radius: var(--radius-sm);
		text-align: left;
		background: transparent;
		border: none;
		cursor: pointer;
		transition: background var(--transition-fast);
	}

	.tree-node:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.tree-node:focus-visible {
		outline: 2px solid var(--color-primary-500);
		outline-offset: -2px;
	}

	.tree-node.modified {
		color: var(--color-warning);
	}

	.tree-node.deleted {
		color: var(--color-error);
		text-decoration: line-through;
	}

	.change-indicator {
		margin-left: auto;
		font-size: var(--text-2xs);
		font-weight: var(--font-medium);
	}

	.change-indicator.modified {
		color: var(--color-warning);
	}

	.change-indicator.added {
		color: var(--color-success);
	}

	.change-indicator.deleted {
		color: var(--color-error);
	}

	.empty-state {
		padding: var(--space-4);
		text-align: center;
		color: var(--color-text-muted);
		font-size: var(--text-xs);
	}
</style>
