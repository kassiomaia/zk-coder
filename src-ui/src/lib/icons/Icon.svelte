<script lang="ts">
    /**
     * Icon Component - Base wrapper for SVG icons
     *
     * Provides consistent sizing, styling, and accessibility for all icons.
     * Icons should be 24x24 viewbox by default (Feather Icons standard).
     *
     * Note: This is a base component for creating new icons.
     * For individual icons, use the specific icon components like CopyIcon, EditIcon, etc.
     */
    import type {SVGAttributes} from 'svelte/elements';

    interface Props {
    /** Icon width in pixels */
    size?: number;
    /** Additional CSS classes */
    class?: string;
    /** SVG stroke width (default: 2) */
    strokeWidth?: number;
    /** Whether to apply currentColor (default: true) */
    colored?: boolean;
    /** ARIA label for accessibility */
    'aria-label'?: string;
    /** ARIA hidden attribute */
    'aria-hidden'?: SVGAttributes<SVGSVGElement>['aria-hidden'];
    /** HTML id attribute */
    id?: string;
    /** Children snippet for SVG content */
    children?: import('svelte').Snippet;
  }

  let {
    size = 24,
    class: className = '',
    strokeWidth = 2,
    colored = true,
    'aria-label': ariaLabel,
    'aria-hidden': ariaHidden,
    id,
    children
  }: Props = $props();

  let stroke = $derived(colored ? 'currentColor' : 'none');
</script>

{#if children}
  {@render children()}
{/if}
