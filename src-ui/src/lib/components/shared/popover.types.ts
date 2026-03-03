export type PopoverOrigin = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export interface PopoverItem {
	id: string;
	label: string;
	iconHtml?: string;
	shortcut?: string;
	variant?: 'default' | 'success' | 'warning' | 'destructive';
	disabled?: boolean;
}
