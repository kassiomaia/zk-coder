<script lang="ts">
	import styles from './FsWriteBody.module.css';
	import InlineDiff from './InlineDiff.svelte';

	type ToolCallStatus = 'pending' | 'success' | 'error';

	interface TouchedFile {
		path: string;
		modified: boolean;
		action: 'read' | 'write' | 'delete';
		diff?: string;
	}

	interface Props {
		args: { filePath?: string; path?: string } | undefined;
		result: unknown;
		status: ToolCallStatus;
		touchedFile?: TouchedFile;
	}

	let { args, result, status, touchedFile }: Props = $props();

	const resolvedPath = $derived(args?.filePath ?? args?.path ?? '');
</script>

{#if status === 'error'}
	<!-- header status indicator handles error display -->
{:else if touchedFile?.diff}
	<InlineDiff diff={touchedFile.diff} filePath={resolvedPath} />
{:else if touchedFile && !touchedFile.diff}
	<div class={styles.notice}>File modified — diff not available</div>
{:else if status === 'pending'}
	<div class={styles.pending}>
		<span class={styles.pendingText}>Applying changes…</span>
	</div>
{:else if status === 'success'}
	<div class={styles.notice}>Changes applied</div>
{/if}
