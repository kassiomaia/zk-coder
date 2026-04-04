<script lang="ts">
	import type { IconProps } from './icons.d';

	let {
		size = 24,
		class: className = '',
		strokeWidth = 2,
		'aria-label': ariaLabel,
		'aria-hidden': ariaHidden
	}: IconProps = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-label={ariaLabel}
	aria-hidden={ariaHidden}
>
	<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
	<circle cx="12" cy="7" r="4" />
</svg>
