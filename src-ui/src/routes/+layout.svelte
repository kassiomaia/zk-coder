<script lang="ts">
	/**
	 * Root Layout
	 * Sets up global styles, theme, and app structure.
	 */
	import '$lib/styles/global.css';
	import { onMount } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';
	import { initTheme, theme as themeStore } from '$lib/stores/themeStore';

	/**
	 * Theme colors from the design system.
	 * These match the CSS variables in design-system.css and are surfaced to
	 * the OS via <meta name="theme-color"> so the chromeless window's frame
	 * blends with the in-app palette.
	 */
	const THEME_COLORS = {
		dark: '#0b0b0a', // --color-neutral-950 (warm)
		light: '#f7f6f3' // --color-bg-base (pale warm)
	} as const;

	let { children } = $props();

	let resolvedTheme = $state<'light' | 'dark'>('light');

	onMount(() => {
		const unsubscribeTheme = themeStore.subscribe((value) => {
			resolvedTheme = value;
		});
		const teardownPromise = initTheme();
		return () => {
			unsubscribeTheme();
			teardownPromise.then((teardown) => teardown()).catch(() => {});
		};
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
	<meta name="theme-color" content={THEME_COLORS[resolvedTheme]} />
	<!-- Transparent background for chromeless window with rounded corners -->
	<style>
		html,
		body {
			background: transparent !important;
		}
	</style>
</svelte:head>

<div class="app" data-theme={resolvedTheme}>
	{@render children()}
</div>

<style>
	.app {
		width: 100%;
		height: 100%;
		overflow: hidden;
		/* Rounded corners for chromeless window */
		border-radius: var(--radius-xl);
		/* Subtle border for window definition */
		border: 1px solid var(--color-border-subtle);
		/* Subtle shadow for depth */
		box-shadow: var(--shadow-2xl);
		/* Ensure content clips to rounded corners */
		overflow: hidden;
	}
</style>
