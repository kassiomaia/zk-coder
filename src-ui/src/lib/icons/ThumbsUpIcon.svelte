<script lang="ts">
	import type { IconProps } from './icons.d';

	interface ThumbsUpProps extends IconProps {
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
	}: ThumbsUpProps = $props();

	let fill = $derived(filled ? 'currentColor' : 'none');
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 24 24"
	{fill}
	stroke="currentColor"
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	class={className}
	aria-label={ariaLabel}
	aria-hidden={ariaHidden}
>
	<path
		d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3zM7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3"
	/>
</svg>
