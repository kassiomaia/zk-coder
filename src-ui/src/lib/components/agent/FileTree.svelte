<script lang="ts">
	/**
	 * FileTree Component
	 * Renders the full workspace as a hierarchical tree, with an overlay
	 * of agent-touched files (M/A/D badges and status color).
	 */
	import { ChevronRight, File, FileCode, FileText, Folder, FolderOpen } from 'lucide-svelte';
	import type { TouchedFile, WorkspaceEntry } from '$lib/types/agent';
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
		allFiles: WorkspaceEntry[];
		touched: TouchedFile[];
		truncated?: boolean;
		onfileclick?: (path: string) => void;
		activePath?: string;
	}

	let { allFiles, touched, truncated = false, onfileclick, activePath }: Props = $props();

	let expandedPaths = $state<Set<string>>(new Set());

	let touchedByPath = $derived.by(() => {
		const m = new Map<string, TouchedFile>();
		for (const f of touched) m.set(f.path, f);
		return m;
	});

	let fileTree = $derived(buildFileTree(allFiles, touched, touchedByPath));

	function buildFileTree(
		workspaceEntries: WorkspaceEntry[],
		touchedFiles: TouchedFile[],
		touchedMap: Map<string, TouchedFile>
	): FileTreeNode {
		const root: FileTreeNode = {
			name: '',
			path: '',
			type: 'directory',
			children: [],
			depth: 0
		};

		for (const entry of workspaceEntries) {
			insertPath(root, entry.path, entry.kind, touchedMap);
		}

		// Synthetic merge: touched paths not in the workspace listing
		// (new files, external writes) still surface with their badge.
		const existing = new Set(workspaceEntries.map((e) => e.path));
		for (const t of touchedFiles) {
			if (!existing.has(t.path)) {
				insertPath(root, t.path, 'file', touchedMap);
			}
		}

		sortTree(root);
		return root;
	}

	function sortTree(node: FileTreeNode) {
		node.children.sort(compareNodes);
		for (const child of node.children) {
			if (child.type === 'directory') sortTree(child);
		}
	}

	function compareNodes(a: FileTreeNode, b: FileTreeNode): number {
		if (a.type !== b.type) return a.type === 'directory' ? -1 : 1;
		return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
	}

	function insertPath(
		root: FileTreeNode,
		path: string,
		leafKind: 'file' | 'directory',
		touchedMap: Map<string, TouchedFile>
	) {
		const parts = path.split('/').filter((p) => p.length > 0);
		if (parts.length === 0) return;
		let current = root;

		for (let i = 0; i < parts.length; i++) {
			const part = parts[i];
			const isLeaf = i === parts.length - 1;
			const segmentPath = parts.slice(0, i + 1).join('/');
			let child = current.children.find((c) => c.name === part);

			if (!child) {
				const nodeType: 'file' | 'directory' = isLeaf ? leafKind : 'directory';
				child = {
					name: part,
					path: segmentPath,
					type: nodeType,
					children: [],
					depth: i,
					file: nodeType === 'file' ? touchedMap.get(segmentPath) : undefined
				};
				current.children.push(child);
			}

			if (child.type === 'directory') {
				current = child;
			}
		}
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
		{#if truncated}
			<div class={styles.truncationFooter}>Showing first 10,000 entries</div>
		{/if}
	{:else}
		<EmptyState icon={FileCode} message="No files to display" class={styles.emptyState} />
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

			{#if isExpanded && node.children.length > 0}
				{@render TreeNode(node.children)}
			{/if}
		{:else}
			{@const indicator = node.file ? getChangeIndicator(node.file) : null}
			{@const FileIcon = getFileIcon(node.path)}
			{@const iconColor = getFileIconColor(node.path)}
			{@const depthClass = node.depth > 5 ? 'depth-5' : `depth-${node.depth}`}
			<button
				class={[
					styles.treeNode,
					styles.file,
					styles[depthClass],
					activePath === node.path && styles.isActive,
					node.file?.action === 'delete' && styles.isDeleted
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
