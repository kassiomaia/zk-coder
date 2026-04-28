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
	<rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
	<line x1="8" y1="21" x2="16" y2="21" />
	<line x1="12" y1="17" x2="12" y2="21" />
</svg>
