<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';

	let appInfo = $state<{ name: string; version: string } | null>(null);
	let status = $state('Connecting to backend...');

	onMount(async () => {
		try {
			const health = await invoke<string>('health_check');
			status = health;
			const info = await invoke<{ name: string; version: string }>('get_app_info');
			appInfo = info;
		} catch (error) {
			status = `Error: ${error}`;
		}
	});
</script>

<main>
	<h1>Welcome to Coder</h1>
	<p class="status">{status}</p>
	{#if appInfo}
		<p class="info">{appInfo.name} v{appInfo.version}</p>
	{/if}
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		padding: var(--space-8);
	}

	h1 {
		font-size: var(--text-4xl);
		font-weight: var(--font-bold);
		margin-bottom: var(--space-4);
		background: linear-gradient(135deg, var(--color-primary-400), var(--color-accent-violet));
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.status {
		font-size: var(--text-lg);
		color: var(--color-text-secondary);
		margin-bottom: var(--space-2);
	}

	.info {
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-text-muted);
		padding: var(--space-2) var(--space-4);
		background-color: var(--color-bg-secondary);
		border-radius: var(--radius-md);
	}
</style>
