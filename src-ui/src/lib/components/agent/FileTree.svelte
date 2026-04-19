<script lang="ts">
	/**
	 * FileTree Component
	 *
	 * Lazy, VS Code-style tree: only the children of an expanded directory
	 * are fetched. The initial render seeds the root from props; deeper
	 * directories load on demand via `list_workspace_dir`. Touched files
	 * (agent M/A/D overlay) auto-expand their ancestors after a short
	 * debounce unless the user has explicitly collapsed them.
	 */
	import { ChevronRight, File, FileCode, FileText, Folder, FolderOpen } from 'lucide-svelte';
	import { untrack } from 'svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import type { TouchedFile, WorkspaceEntry } from '$lib/types/agent';
	import { listWorkspaceDir } from '$lib/api/workspace';
	import EmptyState from '$lib/components/shared/EmptyState.svelte';
	import { createLogger } from '$lib/utils/logger';
	import styles from './FileTree.module.css';

	type DirState =
		| { status: 'loaded'; entries: WorkspaceEntry[]; truncated: boolean }
		| { status: 'loading' }
		| { status: 'error'; message: string };

	interface Props {
		workspaceRoot: string;
		rootEntries: WorkspaceEntry[];
		rootTruncated: boolean;
		touched: TouchedFile[];
		/** Bumped by the parent to invalidate the per-directory cache
		 *  (e.g. on the refresh button). */
		refreshToken?: number;
		onfileclick?: (path: string) => void;
		activePath?: string;
	}

	let {
		workspaceRoot,
		rootEntries,
		rootTruncated,
		touched,
		refreshToken = 0,
		onfileclick,
		activePath
	}: Props = $props();

	const log = createLogger('FileTree');

	/** Cap mirrors `MAX_ENTRIES_PER_DIR` in the Rust command. */
	const DIR_CAP = 1000;
	/** Debounce window for touched-file auto-expand, in milliseconds.
	 *  Bursts of FileTouched events inside one agent step collapse into
	 *  a single expansion + cache-invalidation pass. */
	const TOUCHED_DEBOUNCE_MS = 200;

	const dirCache = new SvelteMap<string, DirState>();
	const expanded = new SvelteSet<string>();
	const userCollapsed = new SvelteSet<string>();

	// Seed the root cache from props; re-runs whenever the parent hands
	// us a new root listing (identity change).
	$effect(() => {
		const entries = rootEntries;
		const truncated = rootTruncated;
		untrack(() => {
			dirCache.set('', { status: 'loaded', entries, truncated });
		});
	});

	// Refresh: clear every cached directory except the root (which the
	// parent just reloaded) and forget user-collapsed state so the tree
	// starts fresh.
	$effect(() => {
		refreshToken;
		untrack(() => {
			const root = dirCache.get('');
			dirCache.clear();
			if (root) dirCache.set('', root);
			userCollapsed.clear();
		});
	});

	// O(1) badge lookup by path.
	let touchedByPath = $derived.by(() => {
		const m = new Map<string, TouchedFile>();
		for (const t of touched) m.set(t.path, t);
		return m;
	});

	// Auto-expand ancestors of touched files + invalidate caches for
	// create/delete actions. Debounced so a FileTouched burst collapses
	// into one pass.
	let expandTimer: ReturnType<typeof setTimeout> | null = null;
	$effect(() => {
		const list = touched;
		untrack(() => {
			if (expandTimer !== null) clearTimeout(expandTimer);
			expandTimer = setTimeout(() => {
				expandTimer = null;
				handleTouchedBurst(list);
			}, TOUCHED_DEBOUNCE_MS);
		});
		return () => {
			if (expandTimer !== null) {
				clearTimeout(expandTimer);
				expandTimer = null;
			}
		};
	});

	function handleTouchedBurst(list: TouchedFile[]) {
		if (!workspaceRoot) return;

		const ancestorsToExpand = new Set<string>();
		const dirsToInvalidate = new Set<string>();

		for (const t of list) {
			const parts = t.path.split('/').filter((p) => p.length > 0);
			if (parts.length === 0) continue;

			// Every ancestor directory except the leaf.
			for (let i = 0; i < parts.length - 1; i++) {
				const ancestor = parts.slice(0, i + 1).join('/');
				if (!userCollapsed.has(ancestor)) ancestorsToExpand.add(ancestor);
			}

			// On create/delete, every directory along the path may have
			// gained or lost a child. Invalidate all of them.
			const isCreate = t.action === 'write' && t.modified === false;
			const isDelete = t.action === 'delete';
			if (isCreate || isDelete) {
				// Every ancestor **except the workspace root** may have
				// gained or lost a child. The root is authoritative from
				// the parent (via `loadWorkspaceRoot` and `refreshToken`);
				// dropping it here would race the parent's seed.
				for (let i = 1; i <= parts.length - 1; i++) {
					dirsToInvalidate.add(parts.slice(0, i).join('/'));
				}
			}
		}

		for (const dir of dirsToInvalidate) {
			dirCache.delete(dir);
		}

		for (const a of ancestorsToExpand) {
			expanded.add(a);
		}

		// Reload directories that are either the root or currently
		// expanded — skip invalidated directories the user can't see,
		// they'll be fetched fresh if they ever expand them.
		// Load any currently-expanded directory whose cache was just
		// invalidated or whose cache is empty (e.g. a freshly auto-
		// expanded ancestor).
		const dirsToLoad = new Set<string>();
		for (const e of expanded) dirsToLoad.add(e);
		for (const d of dirsToInvalidate) {
			if (expanded.has(d)) dirsToLoad.add(d);
		}
		for (const path of dirsToLoad) {
			if (!dirCache.has(path)) {
				void ensureLoaded(path);
			}
		}
	}

	async function ensureLoaded(path: string) {
		if (!workspaceRoot) return;
		const current = dirCache.get(path);
		if (current?.status === 'loaded' || current?.status === 'loading') return;
		dirCache.set(path, { status: 'loading' });
		try {
			const resp = await listWorkspaceDir(workspaceRoot, path);
			dirCache.set(path, {
				status: 'loaded',
				entries: resp.entries,
				truncated: resp.truncated
			});
		} catch (error) {
			log.warn('Failed to list directory', { path, error });
			dirCache.set(path, { status: 'error', message: String(error) });
		}
	}

	function toggle(path: string) {
		if (expanded.has(path)) {
			expanded.delete(path);
			userCollapsed.add(path);
		} else {
			userCollapsed.delete(path);
			expanded.add(path);
			void ensureLoaded(path);
		}
	}

	function depthOf(path: string): number {
		if (path.length === 0) return 0;
		return path.split('/').length - 1;
	}

	function basename(path: string): string {
		const parts = path.split('/');
		return parts[parts.length - 1] ?? path;
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

	let rootState = $derived(dirCache.get(''));
</script>

<div class={styles.fileTree} aria-label="File tree">
	{#if rootState?.status === 'loaded'}
		{#if rootState.entries.length > 0}
			{@render Branch(rootState.entries, rootState.truncated)}
		{:else}
			<EmptyState icon={FileCode} message="No files to display" class={styles.emptyState} />
		{/if}
	{:else if rootState?.status === 'loading'}
		<div class={styles.inlineStatus}>Loading…</div>
	{:else if rootState?.status === 'error'}
		<button
			type="button"
			class={[styles.inlineStatus, styles.inlineError].join(' ')}
			onclick={() => void ensureLoaded('')}
		>
			Failed to load — retry
		</button>
	{:else}
		<EmptyState icon={FileCode} message="No files to display" class={styles.emptyState} />
	{/if}
</div>

{#snippet Branch(entries: WorkspaceEntry[], truncated: boolean)}
	{#each entries as entry (entry.path)}
		{@const isDir = entry.kind === 'directory'}
		{@const depth = depthOf(entry.path)}
		{@const depthClass = depth > 5 ? 'depth-5' : `depth-${depth}`}
		{@const isExpanded = expanded.has(entry.path)}
		{@const touchedFile = touchedByPath.get(entry.path)}

		{#if isDir}
			<button
				class={[styles.treeNode, styles.directory, styles[depthClass]]
					.filter(Boolean)
					.join(' ')}
				onclick={() => toggle(entry.path)}
				type="button"
				aria-expanded={isExpanded}
			>
				<span
					class={[styles.chevronWrapper, isExpanded && styles.isExpanded]
						.filter(Boolean)
						.join(' ')}
				>
					<ChevronRight size={12} strokeWidth={2.5} />
				</span>
				<span class={styles.nodeContent}>
					{#if isExpanded}
						<FolderOpen size={14} strokeWidth={1.5} class={styles.folderIcon} />
					{:else}
						<Folder size={14} strokeWidth={1.5} class={styles.folderIcon} />
					{/if}
					<span class={styles.nodeName}>{basename(entry.path)}</span>
				</span>
			</button>

			{#if isExpanded}
				{@const childState = dirCache.get(entry.path)}
				{@const childDepthClass =
					depth + 1 > 5 ? 'depth-5' : `depth-${depth + 1}`}
				{#if childState?.status === 'loaded'}
					{#if childState.entries.length > 0}
						{@render Branch(childState.entries, childState.truncated)}
					{/if}
				{:else if childState?.status === 'loading'}
					<div class={[styles.inlineStatus, styles[childDepthClass]].join(' ')}>Loading…</div>
				{:else if childState?.status === 'error'}
					<button
						type="button"
						class={[styles.inlineStatus, styles.inlineError, styles[childDepthClass]].join(' ')}
						onclick={() => void ensureLoaded(entry.path)}
					>
						Failed to load — retry
					</button>
				{/if}
			{/if}
		{:else}
			{@const indicator = touchedFile ? getChangeIndicator(touchedFile) : null}
			{@const FileIcon = getFileIcon(entry.path)}
			{@const iconColor = getFileIconColor(entry.path)}
			<button
				class={[
					styles.treeNode,
					styles.file,
					styles[depthClass],
					activePath === entry.path && styles.isActive,
					touchedFile?.action === 'delete' && styles.isDeleted
				]
					.filter(Boolean)
					.join(' ')}
				style="--status-color: {indicator?.color ?? 'transparent'}; --icon-color: {iconColor}"
				onclick={() => onfileclick?.(entry.path)}
				type="button"
			>
				<span class={styles.nodeContent}>
					<FileIcon size={14} strokeWidth={1.5} class={styles.fileIcon} />
					<span class={styles.nodeName}>{basename(entry.path)}</span>
				</span>
				{#if indicator}
					<span class={[styles.changeBadge, styles[indicator.cssClass]].filter(Boolean).join(' ')}>
						{indicator.label}
					</span>
				{/if}
			</button>
		{/if}
	{/each}
	{#if truncated}
		<div class={styles.truncationFooter}>
			Showing first {DIR_CAP} of this folder
		</div>
	{/if}
{/snippet}
