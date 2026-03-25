/**
 * Icon Components Type Definitions
 */

export interface IconProps {
	/** Icon size in pixels (default: 24) */
	size?: number;
	/** Additional CSS classes */
	class?: string;
	/** Stroke width (default: 2) */
	strokeWidth?: number;
	/** ARIA label for accessibility */
	'aria-label'?: string;
	/** ARIA hidden attribute */
	'aria-hidden'?: boolean;
	/** HTML id attribute */
	id?: string;
}
