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
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
</svg>
