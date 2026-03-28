<script lang="ts">
	import FileDiffWorkspace from '$lib/components/agent/FileDiffWorkspace.svelte';

	const params = new URLSearchParams(window.location.search);
	const filePath = params.get('filePath') ?? '';
	const workspaceRoot = params.get('workspaceRoot') ?? '';
	const sessionId = params.get('sessionId') ?? '';

	$: hasAllParams = filePath && workspaceRoot && sessionId;
</script>

<svelte:head>
	<title>{filePath ? filePath.split('/').pop() : 'File Diff'}</title>
</svelte:head>

{#if hasAllParams}
	<FileDiffWorkspace {filePath} {workspaceRoot} {sessionId} />
{:else}
	<div class="error-state">
		<p>Missing required parameters. Please close this window and try again.</p>
	</div>
{/if}

<style>
	.error-state {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100vh;
		padding: var(--space-8);
		color: var(--color-text-muted);
		text-align: center;
	}
</style>
