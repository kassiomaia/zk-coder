/**
 * ListItemActions Types
 * Type definitions for the ListItemActions component.
 */

/**
 * Action identifiers for list item actions.
 */
export type ListItemActionId = 'resolve' | 'reopen' | 'rename' | 'archive' | 'delete';

/**
 * Props for the ListItemActions component.
 */
export interface ListItemActionsProps {
	/** Handler for delete action */
	ondelete?: () => void;
	/** Handler for rename action */
	onrename?: () => void;
	/** Handler for archive action */
	onarchive?: () => void;
	/** Handler for resolve/reopen action */
	onresolve?: () => void;
	/** Whether the item is resolved (affects resolve button label) */
	isResolved?: boolean;
	/** Whether the item is archived (affects archive button label) */
	isArchived?: boolean;
	/** Size of the icons (14 or 16 pixels) */
	iconSize?: 14 | 16;
	/** Callback when menu opens */
	onMenuOpen?: () => void;
	/** Callback when menu closes */
	onMenuClose?: () => void;
}
