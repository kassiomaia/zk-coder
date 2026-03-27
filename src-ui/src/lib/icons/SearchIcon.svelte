<script lang="ts">
    import type {IconProps} from './icons.d';

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
        <circle cx="11" cy="11" r="8" />
        <line x1="21" y1="21" x2="16.65" y2="16.65" />
</svg>
