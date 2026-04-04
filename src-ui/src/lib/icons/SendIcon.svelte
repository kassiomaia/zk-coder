<script lang="ts">
	import type { IconProps } from './icons.d';

	interface SendIconProps extends IconProps {
		/** Whether the icon should be filled */
		filled?: boolean;
	}

	let {
		size = 24,
		class: className = '',
		strokeWidth = 2,
		'aria-label': ariaLabel,
		'aria-hidden': ariaHidden,
		filled = false
	}: SendIconProps = $props();

	let fill = $derived(filled ? 'currentColor' : 'none');
	let stroke = $derived(filled ? 'none' : 'currentColor');
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	{fill}
	{stroke}
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-label={ariaLabel}
	aria-hidden={ariaHidden}
>
	<polyline points="22 2 11 13" />
	<polygon points="22 2 15 22 11 13 2 9 22 2" />
</svg>
