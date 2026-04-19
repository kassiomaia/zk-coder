<script lang="ts">
	import styles from './BashBody.module.css';

	type ToolCallStatus = 'pending' | 'success' | 'error';

	interface Props {
		args: { command?: string } | undefined;
		result: { exit_code?: number } | undefined;
		status: ToolCallStatus;
		streamingContent?: string;
	}

	let { args, result, status, streamingContent }: Props = $props();

	const TRUNCATION_THRESHOLD = 2000;

	const isTruncated = $derived(
		streamingContent !== undefined && streamingContent.length > TRUNCATION_THRESHOLD
	);

	const visibleContent = $derived(
		isTruncated ? streamingContent!.slice(0, TRUNCATION_THRESHOLD) : (streamingContent ?? '')
	);

	const sizeKb = $derived(
		isTruncated ? Math.round(streamingContent!.length / 1024 * 10) / 10 : 0
	);

	const hasContent = $derived(streamingContent !== undefined && streamingContent.length > 0);

	const showExitCode = $derived(
		status !== 'pending' && result?.exit_code !== undefined
	);
</script>

{#if hasContent}
	{#if isTruncated}
		<div class={styles.truncationNotice}>
			<svg
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
				width="11"
				height="11"
				aria-hidden="true"
			>
				<circle cx="12" cy="12" r="10"></circle>
				<line x1="12" y1="8" x2="12" y2="12"></line>
				<line x1="12" y1="16" x2="12.01" y2="16"></line>
			</svg>
			Output too large ({sizeKb} KB). Showing first 2 000 chars.
		</div>
	{/if}

	<div class={styles.terminalBox}>
		<pre class={styles.termPre}>{visibleContent}</pre>
		{#if showExitCode}
			<div class={styles.termDivider}></div>
			<span class={styles.termExit}
				>&lt;command exited with code {result!.exit_code}&gt;</span
			>
		{/if}
	</div>
{:else if status === 'pending'}
	<div class={styles.pendingLine}>
		<span class={styles.pendingDots}>Waiting for output</span>
	</div>
{/if}
