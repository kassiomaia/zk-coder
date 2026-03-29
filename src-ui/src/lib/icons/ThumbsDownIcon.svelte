<script lang="ts">
    import type {IconProps} from './icons.d';

    interface ThumbsDownProps extends IconProps {
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
  }: ThumbsDownProps = $props();

  let fill = $derived(filled ? 'currentColor' : 'none');
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={fill}
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class={className}
  aria-label={ariaLabel}
  aria-hidden={ariaHidden}
>
  <path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3zm7-13h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17" />
</svg>
