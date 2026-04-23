/**
 * Command Types
 * Shared types for the slash-command palette. The list of commands is sourced
 * from the backend `prompt_templates` store; this file defines the shape the
 * palette UI consumes plus a few convenience utilities.
 */

/**
 * Known command categories. User-defined templates may use arbitrary category
 * strings; the palette normalizes unknown values into CUSTOM for display.
 */
export enum CommandCategory {
	CODE = 'code',
	CUSTOM = 'custom'
}

/**
 * Represents a command shown in the palette. Backed one-to-one by a
 * `PromptTemplate` row in `coder_config.db`.
 */
export interface Command {
	/** Stable identifier (template UUID). */
	id: string;
	/** Slash slug, rendered as "/{name}". */
	name: string;
	/** Subtitle shown below the name. */
	description: string;
	/** Optional icon key (see CommandPalette.svelte for supported keys). */
	icon?: string;
	/** Category label. Free-form so user categories round-trip. */
	category: string;
	/** Keyboard shortcut hint (unused today; kept for future). */
	shortcut?: string;
	/** Whether the command is currently available. Defaults to true. */
	available?: boolean;
}

/**
 * Props for the CommandPalette component.
 */
export interface CommandPaletteProps {
	query: string;
	onSelect: (command: Command) => void;
	onClose: () => void;
}

/**
 * Filter commands by query string.
 */
export function filterCommands(commands: Command[], query: string): Command[] {
	const lowerQuery = query.toLowerCase().replace('/', '');

	return commands.filter((cmd) => {
		const matchesName = cmd.name.toLowerCase().includes(lowerQuery);
		const matchesDesc = cmd.description.toLowerCase().includes(lowerQuery);
		return matchesName || matchesDesc;
	});
}

/**
 * Group commands by category.
 */
export function groupCommandsByCategory(commands: Command[]): Map<string, Command[]> {
	const grouped = new Map<string, Command[]>();

	for (const cmd of commands) {
		const key = (cmd.category || CommandCategory.CUSTOM).toString();
		const existing = grouped.get(key) ?? [];
		existing.push(cmd);
		grouped.set(key, existing);
	}

	return grouped;
}

/**
 * Display label for a category value. Unknown values are rendered title-cased.
 */
export function getCategoryLabel(category: string): string {
	const normalized = (category || '').toLowerCase();
	const labels: Record<string, string> = {
		code: 'Code',
		custom: 'Custom',
		navigation: 'Navigation',
		conversation: 'Conversation',
		settings: 'Settings',
		git: 'Git',
		ai: 'AI'
	};
	if (labels[normalized]) return labels[normalized];
	if (!category) return 'Other';
	return category
		.split(/[-_\s]+/)
		.map((word) => (word ? word[0].toUpperCase() + word.slice(1).toLowerCase() : word))
		.join(' ');
}
