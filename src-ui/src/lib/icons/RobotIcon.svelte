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
	<line x1="12" y1="3" x2="12" y2="6" />
	<circle cx="12" cy="2.5" r="0.5" fill="currentColor" stroke="none" />
	<rect x="4" y="7" width="16" height="12" rx="2.5" />
	<line x1="4" y1="12" x2="2.5" y2="12" />
	<line x1="20" y1="12" x2="21.5" y2="12" />
	<circle cx="9" cy="12" r="1.25" fill="currentColor" stroke="none" />
	<circle cx="15" cy="12" r="1.25" fill="currentColor" stroke="none" />
	<line x1="9.5" y1="16" x2="14.5" y2="16" />
</svg>
