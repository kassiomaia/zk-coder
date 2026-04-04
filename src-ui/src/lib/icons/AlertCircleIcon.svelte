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
	<circle cx="12" cy="12" r="10" />
	<line x1="12" y1="8" x2="12" y2="12" />
	<line x1="12" y1="16" x2="12.01" y2="16" />
</svg>
