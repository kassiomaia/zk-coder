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
	<polyline points="3 6 5 6 21 6" />
	<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
</svg>
