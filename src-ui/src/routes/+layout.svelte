<script lang="ts">
	/**
	 * Root Layout
	 * Sets up global styles, theme, and app structure.
	 */
	import '$lib/styles/global.css';
	import { onMount } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';

	/**
	 * Theme colors from the design system.
	 * These match the CSS variables in design-system.css.
	 */
	const THEME_COLORS = {
		dark: '#0b0b0a', // --color-neutral-950 (warm)
		light: '#f7f6f3' // --color-bg-base (pale warm)
	} as const;

	let { children } = $props();

	// Theme handling
	let theme = $state<'light' | 'dark'>('light');

	onMount(() => {
		// Check for saved theme preference
		const savedTheme = localStorage.getItem('theme') as 'light' | 'dark' | null;
		if (savedTheme) {
			theme = savedTheme;
		} else if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
			theme = 'dark';
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
	<meta name="theme-color" content={THEME_COLORS[theme]} />
	<!-- Transparent background for chromeless window with rounded corners -->
	<style>
		html,
		body {
			background: transparent !important;
		}
	</style>
</svelte:head>

<div class="app" data-theme={theme}>
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
