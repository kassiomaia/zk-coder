<script lang="ts">
	/**
	 * FileTree Component
	 * Displays files touched by the agent in a hierarchical tree.
	 * Supports recursive rendering of nested directories.
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
		activePath?: string;
	}

	let { files, onfileclick, activePath }: Props = $props();

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
				return { icon: FileCode, color: 'var(--color-primary-400)' };
			case 'rs':
				return { icon: FileCode, color: '#f74c00' };
			case 'py':
				return { icon: FileCode, color: '#3776ab' };
			case 'md':
				return { icon: FileText, color: 'var(--color-text-muted)' };
			case 'toml':
			case 'yaml':
			case 'yml':
			case 'json':
				return { icon: FileText, color: 'var(--color-warning-dark)' };
			default:
				return { icon: File, color: 'var(--color-text-muted)' };
		}
	}

	function getChangeIndicator(file: TouchedFile): { label: string; cssClass: string; color: string } | null {
		if (file.action === 'write' && file.modified) return { label: 'M', cssClass: 'modified', color: 'var(--color-warning)' };
		if (file.action === 'write' && !file.modified) return { label: 'A', cssClass: 'added', color: 'var(--color-success)' };
		if (file.action === 'delete') return { label: 'D', cssClass: 'deleted', color: 'var(--color-error)' };
		return null;
	}

	/** Base indentation for nested items */
	const BASE_INDENT = 12;
	const INDENT_PER_LEVEL = 16;
</script>

<div class="file-tree" aria-label="File tree">
	{#if fileTree.children.length > 0}
		{@render TreeNode(fileTree.children, 0)}
	{:else}
		<div class="empty-state">
			<p>No files changed in this session.</p>
		</div>
	{/if}
</div>

{#snippet TreeNode(nodes: FileTreeNode[], depth: number)}
	{#each nodes as node (node.path)}
		{#if node.type === 'directory'}
			{@const isExpanded = expandedPaths.has(node.path)}
			<button
				class="tree-node directory"
				style="padding-left: {BASE_INDENT + depth * INDENT_PER_LEVEL}px"
				onclick={() => toggleExpanded(node.path)}
				type="button"
				aria-expanded={isExpanded}
			>
				<span class="chevron-wrapper" class:is-expanded={isExpanded}>
					<ChevronRight size={12} strokeWidth={2.5} />
				</span>
				<span class="node-content">
					{#if isExpanded}
						<FolderOpen size={14} strokeWidth={1.5} class="folder-icon" />
					{:else}
						<Folder size={14} strokeWidth={1.5} class="folder-icon" />
					{/if}
					<span class="node-name">{node.name}</span>
				</span>
			</button>

			{#if isExpanded}
				{#if node.children.length > 0}
					{@render TreeNode(node.children, depth + 1)}
				{/if}
			{/if}

		{:else if node.file}
			{@const indicator = getChangeIndicator(node.file)}
			{@const iconData = getFileIcon(node.path)}
			{@const FileIcon = iconData.icon}
			<button
				class="tree-node file"
				class:is-active={activePath === node.path}
				class:is-modified={node.file.action === 'write'}
				class:is-deleted={node.file.action === 'delete'}
				style="padding-left: {BASE_INDENT + depth * INDENT_PER_LEVEL + 16}px; --status-color: {indicator?.color ?? 'transparent'}"
				onclick={() => onfileclick?.(node.path)}
				type="button"
			>
				<span class="node-content">
					<FileIcon size={14} strokeWidth={1.5} style="color: {iconData.color}" />
					<span class="node-name">{node.name}</span>
				</span>
				{#if indicator}
					<span class="change-badge {indicator.cssClass}">{indicator.label}</span>
				{/if}
			</button>
		{/if}
	{/each}
{/snippet}

<style>
	.file-tree {
		padding: var(--space-1) 0;
	}

	.tree-node {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: var(--space-1) var(--space-4);
		font-size: 11px;
		color: var(--color-text-secondary);
		background: transparent;
		border: none;
		cursor: pointer;
		transition: all var(--transition-fast);
		position: relative;
	}

	.tree-node:hover {
		background: var(--color-interactive-hover);
		color: var(--color-text-primary);
	}

	.tree-node.file.is-active {
		background: var(--color-interactive-active);
		color: var(--color-text-primary);
	}

	.tree-node.file.is-active::after {
		content: '';
		position: absolute;
		right: 0;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--color-primary-500);
	}

	.tree-node.file::before {
		content: '';
		position: absolute;
		left: 0;
		top: 2px;
		bottom: 2px;
		width: 2px;
		background: var(--status-color);
		opacity: 0.6;
	}

	.node-content {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
		flex: 1;
	}

	.node-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.chevron-wrapper {
		display: flex;
		align-items: center;
		justify-content: center;
		margin-right: var(--space-1);
		transition: transform var(--transition-fast);
		opacity: 0.5;
	}

	.chevron-wrapper.is-expanded {
		transform: rotate(90deg);
	}

	.folder-icon {
		color: var(--color-primary-400);
		opacity: 0.8;
	}

	.change-badge {
		font-size: 9px;
		font-weight: 700;
		width: 14px;
		height: 14px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 2px;
		margin-left: var(--space-2);
	}

	.change-badge.modified { color: var(--color-warning); background: var(--color-warning-light); }
	.change-badge.added { color: var(--color-success); background: var(--color-success-light); }
	.change-badge.deleted { color: var(--color-error); background: var(--color-error-light); }

	.tree-node.is-deleted .node-name {
		text-decoration: line-through;
		opacity: 0.5;
	}

	.empty-state {
		padding: var(--space-4) var(--space-4) var(--space-6);
		color: var(--color-text-muted);
		font-size: 11px;
		font-style: italic;
	}
</style>
