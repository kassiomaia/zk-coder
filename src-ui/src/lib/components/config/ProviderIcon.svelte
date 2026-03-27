<script lang="ts">
    /**
     * ProviderIcon Component
     * Displays generic icons for AI providers using Lucide icons.
     */
    import type {Provider} from '$lib/types/config';

    interface Props {
		provider: Provider;
		size?: number;
	}

	let { provider, size = 32 }: Props = $props();

	// Provider brand colors - use CSS custom properties for themability
	const colorMap: Record<string, string> = {
		anthropic: 'var(--color-provider-anthropic)',
		openai: 'var(--color-provider-openai)',
		google: 'var(--color-provider-google)'
	};

	let iconColor = $derived(colorMap[provider.id] || 'var(--color-provider-default)');
</script>

{#if provider.id === 'anthropic'}
	<svg
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		stroke={iconColor}
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="M12 8V4H8" />
		<rect width="16" height="12" x="4" y="8" rx="2" />
		<path d="M2 14h2" />
		<path d="M20 14h2" />
		<path d="M15 13v2" />
		<path d="M9 13v2" />
	</svg>
{:else if provider.id === 'openai'}
	<svg
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		stroke={iconColor}
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<rect x="4" y="4" width="16" height="16" rx="2" />
		<rect x="9" y="9" width="6" height="6" />
		<path d="M9 1V4" />
		<path d="M15 1V4" />
		<path d="M9 20V23" />
		<path d="M15 20V23" />
		<path d="M20 9H23" />
		<path d="M20 14H23" />
		<path d="M1 9H4" />
		<path d="M1 14H4" />
	</svg>
{:else if provider.id === 'google'}
	<svg
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		stroke={iconColor}
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<circle cx="12" cy="12" r="10" />
		<path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20" />
		<path d="M2 12h20" />
	</svg>
{:else}
	<svg
		width={size}
		height={size}
		viewBox="0 0 24 24"
		fill="none"
		stroke={iconColor}
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<path d="M12 8V4H8" />
		<rect width="16" height="12" x="4" y="8" rx="2" />
		<path d="M2 14h2" />
		<path d="M20 14h2" />
		<path d="M15 13v2" />
		<path d="M9 13v2" />
	</svg>
{/if}
