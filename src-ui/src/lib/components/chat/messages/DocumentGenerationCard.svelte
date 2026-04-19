<script lang="ts">
	/**
	 * DocumentGenerationCard
	 *
	 * Compact one-row card for generated document artifacts. Shows progress
	 * during generation and morphs into a ready-to-save widget when complete,
	 * without changing height. Matches the product design in
	 * `.prototypes/document-download-card/v1.html`.
	 */
	import styles from './DocumentGenerationCard.module.css';
	import {
		FileText,
		File,
		Presentation,
		CheckCircle2,
		Download,
		AlertCircle,
		RefreshCw,
		X,
		Copy
	} from 'lucide-svelte';
	import { slide } from 'svelte/transition';

	export type DocType = 'Docx' | 'Pdf' | 'Pptx';

	export type GenPhase =
		| 'preparing'
		| 'building'
		| 'finalizing'
		| 'completed'
		| 'failed'
		| 'retrying';

	export interface ProgressStep {
		label: string;
		completed: boolean;
		current?: boolean;
	}

	interface Props {
		docType: DocType;
		filename: string;
		phase: GenPhase;
		statusText: string;
		progress: number | null;
		steps?: ProgressStep[];
		error?: string;
		warnings?: string[];
		expanded?: boolean;
		sessionId?: string;
		artifactId?: string;
		filePath?: string;
		onDownload?: () => void | Promise<void>;
		onCopy?: () => void | Promise<void>;
		onRetry?: () => void;
		onDismiss?: () => void;
	}

	let {
		docType,
		filename,
		phase = 'preparing',
		statusText = 'Starting…',
		progress = null,
		error,
		warnings = [],
		expanded = false,
		filePath,
		onDownload,
		onCopy,
		onRetry,
		onDismiss
	}: Props = $props();

	let internallyExpanded = $state(false);
	let isDownloading = $state(false);
	let showCopied = $state(false);

	$effect(() => {
		internallyExpanded = expanded || false;
	});

	// Use $derived.by so the switch actually runs (plain $derived of an
	// arrow function would store the function itself and leak it to the DOM).
	const FileIcon = $derived.by(() => {
		switch (docType) {
			case 'Docx':
				return FileText;
			case 'Pdf':
				return File;
			case 'Pptx':
				return Presentation;
			default:
				return File;
		}
	});

	const typeLabel = $derived.by(() => {
		switch (docType) {
			case 'Docx':
				return 'Document';
			case 'Pdf':
				return 'PDF';
			case 'Pptx':
				return 'Presentation';
			default:
				return 'File';
		}
	});

	const isComplete = $derived(phase === 'completed');
	const isFailed = $derived(phase === 'failed' || phase === 'retrying');
	const isActive = $derived(!isComplete && !isFailed);
	const hasDeterminateProgress = $derived(progress !== null && progress >= 0 && progress <= 100);
	const progressPct = $derived(hasDeterminateProgress ? Math.max(0, Math.min(100, progress ?? 0)) : 0);
	const hasWarnings = $derived(!error && warnings.length > 0);

	async function handleDownload() {
		if (isDownloading) return;
		isDownloading = true;
		try {
			await onDownload?.();
		} finally {
			setTimeout(() => {
				isDownloading = false;
			}, 400);
		}
	}

	async function handleCopy() {
		try {
			await onCopy?.();
			showCopied = true;
			setTimeout(() => (showCopied = false), 1600);
		} catch {
			// onCopy handler logs upstream; avoid swallowing silently but don't crash the UI
		}
	}

	function toggleExpanded() {
		internallyExpanded = !internallyExpanded;
	}
</script>

<div
	class={[
		styles.card,
		isComplete && styles.complete,
		isFailed && styles.failed,
		isActive && styles.active
	]}
	role="status"
	aria-live="polite"
	aria-label={isComplete ? `Document ${filename} ready` : `Generating ${filename}`}
>
	<div class={styles.main}>
		<div class={[styles.tile, styles[docType], isFailed && styles.tileFailed]}>
			{#if isFailed}
				<AlertCircle size={20} />
			{:else}
				<FileIcon size={20} strokeWidth={1.6} />
			{/if}
		</div>

		<div class={styles.identity}>
			<div class={styles.filename} title={filename}>{filename}</div>
			<div class={styles.subline}>
				{#if isComplete}
					<span>{typeLabel}</span>
					<span class={styles.dot} aria-hidden="true"></span>
					<span class={styles.ready}>
						<CheckCircle2 size={12} strokeWidth={2} />
						Ready
					</span>
				{:else if isFailed}
					<span class={styles.errorText}>
						{phase === 'retrying' ? 'Retrying…' : (error ?? 'Generation failed')}
					</span>
				{:else}
					<span class={styles.spinner} aria-hidden="true"></span>
					<span class={styles.statusText}>
						{statusText}{#if hasDeterminateProgress}&nbsp;·&nbsp;{progressPct}%{/if}
					</span>
				{/if}
			</div>
		</div>

		<div class={styles.actions}>
			{#if isComplete}
				{#if onCopy && filePath}
					<button
						class={styles.iconBtn}
						onclick={handleCopy}
						title={showCopied ? 'Copied' : 'Copy path'}
						aria-label="Copy path"
						type="button"
					>
						{#if showCopied}
							<CheckCircle2 size={14} strokeWidth={2} />
						{:else}
							<Copy size={14} strokeWidth={1.8} />
						{/if}
					</button>
				{/if}
				<button
					class={styles.saveBtn}
					onclick={handleDownload}
					disabled={isDownloading}
					title="Save document"
					type="button"
				>
					{#if isDownloading}
						<RefreshCw size={14} strokeWidth={1.8} class={styles.spin} />
						<span>Saving…</span>
					{:else}
						<Download size={14} strokeWidth={1.8} />
						<span>Save</span>
					{/if}
				</button>
			{:else if isFailed}
				{#if onRetry}
					<button
						class={styles.retryBtn}
						onclick={() => onRetry?.()}
						title="Retry generation"
						type="button"
					>
						<RefreshCw size={14} strokeWidth={1.8} />
						<span>Retry</span>
					</button>
				{/if}
				{#if error && !internallyExpanded}
					<button class={styles.ghostBtn} onclick={toggleExpanded} title="Show details" type="button">
						<span class={styles.ghostLabel}>Details</span>
					</button>
				{/if}
			{:else}
				{#if hasWarnings}
					<button
						class={styles.ghostBtn}
						onclick={toggleExpanded}
						title="Show warnings"
						type="button"
					>
						<AlertCircle size={12} strokeWidth={1.8} />
						<span class={styles.ghostLabel}
							>{warnings.length} warning{warnings.length === 1 ? '' : 's'}</span
						>
					</button>
				{/if}
				{#if onDismiss}
					<button
						class={styles.ghostBtn}
						onclick={() => onDismiss?.()}
						title="Hide"
						aria-label="Hide"
						type="button"
					>
						<X size={14} strokeWidth={1.8} />
					</button>
				{/if}
			{/if}
		</div>
	</div>

	{#if isActive}
		<div class={styles.rail} aria-hidden="true">
			{#if hasDeterminateProgress}
				<div class={styles.railFill} style="width: {progressPct}%"></div>
			{:else}
				<div class={styles.railIndeterminate}></div>
			{/if}
		</div>
	{/if}

	{#if (error || hasWarnings) && internallyExpanded}
		<div class={styles.details} transition:slide={{ duration: 180 }}>
			{#if error}
				<div class={styles.detailsHeader}>Error details</div>
				<pre class={styles.detailsBody}>{error}</pre>
			{:else if hasWarnings}
				<div class={styles.detailsHeader}>
					{warnings.length} warning{warnings.length === 1 ? '' : 's'}
				</div>
				<ul class={styles.detailsList}>
					{#each warnings as warning}
						<li>{warning}</li>
					{/each}
				</ul>
			{/if}
		</div>
	{/if}
</div>
