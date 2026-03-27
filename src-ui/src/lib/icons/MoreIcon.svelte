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
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
        <circle cx="5" cy="12" r="1" />
</svg>
