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
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <polyline points="14 2 14 8 20 8" />
</svg>
