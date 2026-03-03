<script lang="ts">
	/**
	 * Root Layout
	 * Sets up global styles, theme, and app structure.
	 */
	import '$lib/styles/global.css';
	import { onMount } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	// Theme handling
	let theme = $state<'light' | 'dark'>('dark');

	onMount(() => {
		// Check for saved theme preference
		const savedTheme = localStorage.getItem('theme') as 'light' | 'dark' | null;
		if (savedTheme) {
			theme = savedTheme;
		} else if (window.matchMedia('(prefers-color-scheme: light)').matches) {
			theme = 'light';
		}

		// Apply theme
		document.documentElement.setAttribute('data-theme', theme);

		// Listen for system theme changes
		const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const handler = (e: MediaQueryListEvent) => {
			if (!localStorage.getItem('theme')) {
				theme = e.matches ? 'dark' : 'light';
				document.documentElement.setAttribute('data-theme', theme);
			}
		};
		mediaQuery.addEventListener('change', handler);

		return () => mediaQuery.removeEventListener('change', handler);
	});

	function toggleTheme() {
		theme = theme === 'dark' ? 'light' : 'dark';
		document.documentElement.setAttribute('data-theme', theme);
		localStorage.setItem('theme', theme);
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
	<meta name="theme-color" content={theme === 'dark' ? '#111827' : '#ffffff'} />
</svelte:head>

<div class="app" data-theme={theme}>
	{@render children()}
</div>

<style>
	.app {
		width: 100%;
		height: 100%;
		overflow: hidden;
	}
</style>
