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
        <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
        <polyline points="22 4 12 14.01 9 11.01" />
</svg>
