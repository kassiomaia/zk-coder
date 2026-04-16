<script lang="ts">
	/**
	 * DocumentGenerationCard Component
	 *
	 * A unified, live job card for document generation workflows.
	 * Aggregates all tool calls into a single elegant component that shows progress
	 * from start to finish, then morphs into a completed artifact card.
	 *
	 * Lifecycle:
	 * 1. preparing -> Document generation started
	 * 2. building -> Adding content, sections, slides
	 * 3. finalizing -> Assembling and finalizing the file
	 * 4. completed -> Ready to download
	 * 5. failed -> Error occurred
	 *
	 * File types: DOCX, PDF, PPTX
	 */
	import styles from './DocumentGenerationCard.module.css';
	import { onMount } from 'svelte';
	import {
		FileText,
		File,
		Presentation,
		CheckCircle2,
		Download,
		ExternalLink,
		AlertCircle,
		RefreshCw,
		X,
		Copy
	} from 'lucide-svelte';
	import { fade, slide, scale } from 'svelte/transition';

	/** Document type */
	export type DocType = 'Docx' | 'Pdf' | 'Pptx';

	/** Generation phase */
	export type GenPhase =
		| 'preparing'
		| 'building'
		| 'finalizing'
		| 'completed'
		| 'failed'
		| 'retrying';

	/** Progress step for display */
	export interface ProgressStep {
		label: string;
		completed: boolean;
		current?: boolean;
	}

	interface Props {
		/** Type of document being generated */
		docType: DocType;
		/** Filename of the document */
		filename: string;
		/** Current generation phase */
		phase: GenPhase;
		/** Human-readable status text */
		statusText: string;
		/** Progress percentage (0-100), or null for indeterminate */
		progress: number | null;
		/** Individual progress steps to display */
		steps?: ProgressStep[];
		/** Error message if failed */
		error?: string;
		/** Non-fatal warnings from the renderer (T3.2). */
		warnings?: string[];
		/** Whether details are expanded */
		expanded?: boolean;
		/** Session ID for this generation */
		sessionId?: string;
		/** Artifact ID when completed */
		artifactId?: string;
		/** File path when completed */
		filePath?: string;
		/** Download handler */
		onDownload?: () => void | Promise<void>;
		/** Copy link handler */
		onCopy?: () => void | Promise<void>;
		/** Retry handler */
		onRetry?: () => void;
		/** Dismiss handler */
		onDismiss?: () => void;
	}

	let {
		docType,
		filename,
		phase = 'preparing',
		statusText = 'Starting...',
		progress = null,
		steps = [],
		error,
		warnings = [],
		expanded = false,
		sessionId,
		artifactId,
		filePath,
		onDownload,
		onCopy,
		onRetry,
		onDismiss
	}: Props = $props();

	// Internal state
	let internallyExpanded = $state(false);
	let isDownloading = $state(false);
	let isCopying = $state(false);
	let showCopied = $state(false);

	// Track expanded state - use prop value, but allow local override when toggled
	// Initialize with prop value
	$effect(() => {
		internallyExpanded = expanded || false;
	});

	// Computed value for whether currently expanded
	let isExpanded = $derived(internallyExpanded);

	// Animation state for progress bar
	let progressWidth = $state(0);
	let animationFrame: number;

	// Animate progress bar smoothly
	$effect(() => {
		if (progress === null) {
			progressWidth = 100;
			return;
		}

		const targetWidth = progress;
		const duration = 300;
		const startTime = performance.now();
		const startWidth = progressWidth;

		const animate = (now: number) => {
			const elapsed = now - startTime;
			const t = Math.min(elapsed / duration, 1);
			// Ease out cubic
			const eased = 1 - Math.pow(1 - t, 3);
			progressWidth = startWidth + (targetWidth - startWidth) * eased;

			if (t < 1) {
				animationFrame = requestAnimationFrame(animate);
			}
		};

		animationFrame = requestAnimationFrame(animate);

		return () => cancelAnimationFrame(animationFrame);
	});

	// Get file icon component
	const FileIcon = $derived(() => {
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

	// Get file type label
	const fileLabel = $derived(() => {
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

	// Get file extension for badge
	const fileExtension = $derived(() => {
		switch (docType) {
			case 'Docx':
				return 'DOCX';
			case 'Pdf':
				return 'PDF';
			case 'Pptx':
				return 'PPTX';
			default:
				return docType;
		}
	});

	// Check if generation is complete
	const isComplete = $derived(phase === 'completed');
	const isFailed = $derived(phase === 'failed' || phase === 'retrying');
	const isActive = $derived(!isComplete && !isFailed);

	// Get status icon
	const statusIcon = $derived(() => {
		if (isComplete) return CheckCircle2;
		if (isFailed) return AlertCircle;
		return RefreshCw;
	});

	// Handle download
	async function handleDownload() {
		if (isDownloading) return;
		isDownloading = true;
		try {
			await onDownload?.();
		} finally {
			setTimeout(() => {
				isDownloading = false;
			}, 500);
		}
	}

	// Handle copy
	async function handleCopy() {
		if (isCopying) return;
		isCopying = true;
		try {
			await onCopy?.();
			showCopied = true;
			setTimeout(() => (showCopied = false), 2000);
		} finally {
			isCopying = false;
		}
	}

	// Handle dismiss
	function handleDismiss() {
		onDismiss?.();
	}

	// Handle retry
	function handleRetry() {
		onRetry?.();
	}

	// Toggle expanded
	function toggleExpanded() {
		internallyExpanded = !internallyExpanded;
	}

	// Format file size
	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	// Cleanup animation on unmount
	onMount(() => {
		return () => {
			if (animationFrame) {
				cancelAnimationFrame(animationFrame);
			}
		};
	});
</script>

<div
	class={[
		styles.card,
		styles[phase],
		isComplete && styles.complete,
		isFailed && styles.failed,
		isActive && styles.active
	]}
	role="status"
	aria-live="polite"
	aria-label={isComplete ? `Document ${filename} ready` : `Generating ${filename}`}
>
	<!-- Main card content -->
	<div class={styles.main}>
		<!-- File identity area -->
		<div class={styles.identity}>
			<div class={[styles.iconWrapper, styles[docType], isActive && styles.animated].join(' ')}>
				<FileIcon size={24} class={styles.fileIcon} />
				{#if isActive}
					<div class={styles.iconGlow}></div>
				{/if}
			</div>
			<div class={styles.fileInfo}>
				<div class={styles.filename}>{filename}</div>
				<div class={styles.fileMeta}>
					<span class={styles.fileType}>{fileLabel}</span>
					<span class={styles.separator}>·</span>
					<span class={styles.extBadge}>{fileExtension}</span>
					{#if isComplete && filePath}
						<span class={styles.separator}>·</span>
						<span class={styles.readyBadge}>Ready</span>
					{/if}
				</div>
			</div>
		</div>

		<!-- Status and progress area -->
		<div class={styles.statusArea}>
			{#if isActive}
				<!-- Active generation state -->
				<div class={styles.activeStatus}>
					<div class={styles.statusLine}>
						<RefreshCw size={14} class={styles.spinnerIcon} />
						<span class={styles.statusText}>{statusText}</span>
					</div>

					<!-- Progress bar -->
					{#if progress !== null}
						<div class={styles.progressBar}>
							<div class={styles.progressFill} style="width: {progressWidth}%"></div>
							{#if progress >= 0 && progress <= 100}
								<span class={styles.progressPercent}>{Math.round(progress)}%</span>
							{/if}
						</div>
					{:else}
						<!-- Indeterminate progress -->
						<div class={styles.progressIndeterminate}>
							<div class={styles.indeterminateBar}></div>
						</div>
					{/if}

					<!-- Progress steps -->
					{#if steps && steps.length > 0}
						<div class={styles.stepsList}>
							{#each steps as step, i}
								<div
									class={[
										styles.step,
										step.completed && styles.stepCompleted,
										step.current && styles.stepCurrent
									]}
								>
									{#if step.completed}
										<CheckCircle2 size={12} class={styles.stepIcon} />
									{:else if step.current}
										<RefreshCw size={12} class={styles.stepIcon} />
									{:else}
										<div class={styles.stepDot}></div>
									{/if}
									<span class={styles.stepLabel}>{step.label}</span>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{:else if isFailed}
				<!-- Failed state -->
				<div class={styles.failedStatus}>
					<div class={styles.statusLine}>
						<AlertCircle size={14} class={styles.errorIcon} />
						<span class={styles.statusText}>
							{phase === 'retrying' ? 'Retrying...' : 'Generation failed'}
						</span>
					</div>
					{#if error && isExpanded}
						<div class={styles.errorMessage}>{error}</div>
					{/if}
				</div>
			{:else if isComplete}
				<!-- Completed state -->
				<div class={styles.completeStatus}>
					<div class={styles.statusLine}>
						<CheckCircle2 size={14} class={styles.successIcon} />
						<span class={styles.statusText}>Document ready</span>
					</div>
				</div>
			{/if}
		</div>

		<!-- Actions area -->
		<div class={styles.actions}>
			{#if isComplete}
				<!-- Completed actions -->
				<button
					class={[styles.actionBtn, styles.primary].join(' ')}
					onclick={handleDownload}
					disabled={isDownloading}
					title="Download document"
					type="button"
				>
					{#if isDownloading}
						<RefreshCw size={16} class={styles.spinning} />
						<span>Downloading...</span>
					{:else}
						<Download size={16} />
						<span>Download</span>
					{/if}
				</button>

				{#if onCopy}
					<button
						class={[styles.actionBtn, styles.secondary].join(' ')}
						onclick={handleCopy}
						disabled={isCopying}
						title="Copy link"
						type="button"
					>
						{#if showCopied}
							<CheckCircle2 size={16} />
							<span>Copied</span>
						{:else}
							<Copy size={16} />
							<span>Copy</span>
						{/if}
					</button>
				{/if}

				{#if filePath}
					<button
						class={[styles.actionBtn, styles.secondary].join(' ')}
						title="Open file location"
						type="button"
					>
						<ExternalLink size={16} />
					</button>
				{/if}
			{:else if isFailed && onRetry}
				<!-- Failed actions -->
				<button
					class={[styles.actionBtn, styles.secondary].join(' ')}
					onclick={handleRetry}
					title="Retry generation"
					type="button"
				>
					<RefreshCw size={16} />
					<span>Retry</span>
				</button>
			{:else if isActive && onDismiss}
				<!-- Active: show dismiss to hide -->
				<button
					class={[styles.actionBtn, styles.ghost].join(' ')}
					onclick={handleDismiss}
					title="Hide"
					type="button"
				>
					<X size={16} />
				</button>
			{/if}

			{#if error && !isExpanded}
				<button
					class={[styles.actionBtn, styles.ghost].join(' ')}
					onclick={toggleExpanded}
					title="Show details"
					type="button"
				>
					<span class={styles.expandText}>Details</span>
				</button>
			{/if}

			{#if !error && warnings && warnings.length > 0 && !isExpanded}
				<button
					class={[styles.actionBtn, styles.ghost].join(' ')}
					onclick={toggleExpanded}
					title="Show warnings"
					type="button"
				>
					<AlertCircle size={12} />
					<span class={styles.expandText}>{warnings.length} warning{warnings.length === 1 ? '' : 's'}</span>
				</button>
			{/if}
		</div>
	</div>

	<!-- Expanded error details -->
	{#if error && isExpanded}
		<div class={styles.details} transition:slide={{ duration: 200 }}>
			<div class={styles.errorDetails}>
				<div class={styles.errorHeader}>Error details</div>
				<pre class={styles.errorText}>{error}</pre>
			</div>
		</div>
	{/if}

	<!-- Expanded warnings (T3.2) -->
	{#if !error && warnings && warnings.length > 0 && isExpanded}
		<div class={styles.details} transition:slide={{ duration: 200 }}>
			<div class={styles.errorDetails}>
				<div class={styles.errorHeader}>{warnings.length} warning{warnings.length === 1 ? '' : 's'}</div>
				<ul class={styles.warningsList}>
					{#each warnings as warning}
						<li>{warning}</li>
					{/each}
				</ul>
			</div>
		</div>
	{/if}
</div>

<style>
	/* Imported from module CSS for cleaner separation */
</style>
