<script lang="ts">
	/**
	 * FileTree Component
	 * Displays files touched by the agent in a hierarchical tree.
	 * Supports recursive rendering of nested directories.
	 */
	import { ChevronRight, File, FileCode, FileText, Folder, FolderOpen, HardDrive } from 'lucide-svelte';
	import type { TouchedFile } from '$lib/types/agent';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import styles from './FileTree.module.css';

	interface FileTreeNode {
		name: string;
		path: string;
		type: 'file' | 'directory';
		children: FileTreeNode[];
		file?: TouchedFile;
		depth: number;
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
		const root: FileTreeNode = { name: '', path: '', type: 'directory', children: [], depth: 0 };

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
						file: isFile ? file : undefined,
						depth: i
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

	function getFileIconColor(filePath: string): string {
		const ext = filePath.split('.').pop()?.toLowerCase();
		switch (ext) {
			case 'ts':
			case 'js':
			case 'tsx':
			case 'jsx':
				return 'var(--color-lang-typescript)';
			case 'rs':
				return 'var(--color-lang-rust)';
			case 'py':
				return 'var(--color-lang-python)';
			case 'md':
				return 'var(--color-lang-default)';
			case 'toml':
			case 'yaml':
			case 'yml':
			case 'json':
				return 'var(--color-warning-dark)';
			default:
				return 'var(--color-lang-default)';
		}
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
				return FileCode;
			case 'md':
			case 'toml':
			case 'yaml':
			case 'yml':
			case 'json':
				return FileText;
			default:
				return File;
		}
	}

	function getChangeIndicator(
		file: TouchedFile
	): { label: string; cssClass: string; color: string } | null {
		if (file.action === 'write' && file.modified)
			return { label: 'M', cssClass: 'modified', color: 'var(--color-warning)' };
		if (file.action === 'write' && !file.modified)
			return { label: 'A', cssClass: 'added', color: 'var(--color-success)' };
		if (file.action === 'delete')
			return { label: 'D', cssClass: 'deleted', color: 'var(--color-error)' };
		return null;
	}
</script>

<div class={styles.fileTree} aria-label="File tree">
	{#if fileTree.children.length > 0}
		{@render TreeNode(fileTree.children)}
	{:else}
		<EmptyState icon={FileCode} message="No files modified" class={styles.emptyState} />
	{/if}
</div>

{#snippet TreeNode(nodes: FileTreeNode[])}
	{#each nodes as node (node.path)}
		{#if node.type === 'directory'}
			{@const isExpanded = expandedPaths.has(node.path)}
			{@const depthClass = node.depth > 5 ? 'depth-5' : `depth-${node.depth}`}
			<button
				class={[styles.treeNode, styles.directory, styles[depthClass]].filter(Boolean).join(' ')}
				onclick={() => toggleExpanded(node.path)}
				type="button"
				aria-expanded={isExpanded}
			>
				<span
					class={[styles.chevronWrapper, isExpanded && styles.isExpanded].filter(Boolean).join(' ')}
				>
					<ChevronRight size={12} strokeWidth={2.5} />
				</span>
				<span class={styles.nodeContent}>
					{#if isExpanded}
						<FolderOpen size={14} strokeWidth={1.5} class={styles.folderIcon} />
					{:else}
						<Folder size={14} strokeWidth={1.5} class={styles.folderIcon} />
					{/if}
					<span class={styles.nodeName}>{node.name}</span>
				</span>
			</button>

			{#if isExpanded}
				{#if node.children.length > 0}
					{@render TreeNode(node.children)}
				{/if}
			{/if}
		{:else if node.file}
			{@const indicator = getChangeIndicator(node.file)}
			{@const FileIcon = getFileIcon(node.path)}
			{@const iconColor = getFileIconColor(node.path)}
			{@const depthClass = node.depth > 5 ? 'depth-5' : `depth-${node.depth}`}
			<button
				class={[
					styles.treeNode,
					styles.file,
					styles[depthClass],
					activePath === node.path && styles.isActive,
					node.file.action === 'delete' && styles.isDeleted
				]
					.filter(Boolean)
					.join(' ')}
				style="--status-color: {indicator?.color ?? 'transparent'}; --icon-color: {iconColor}"
				onclick={() => onfileclick?.(node.path)}
				type="button"
			>
				<span class={styles.nodeContent}>
					<FileIcon size={14} strokeWidth={1.5} class={styles.fileIcon} />
					<span class={styles.nodeName}>{node.name}</span>
				</span>
				{#if indicator}
					<span class={[styles.changeBadge, styles[indicator.cssClass]].filter(Boolean).join(' ')}>
						{indicator.label}
					</span>
				{/if}
			</button>
		{/if}
	{/each}
{/snippet}
