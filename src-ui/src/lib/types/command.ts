/**
 * Command Types
 * Type definitions for the command palette system in the Coder AI application.
 */

/**
 * Categories for organizing commands.
 */
export enum CommandCategory {
	CODE = 'code',
	NAVIGATION = 'navigation',
	CONVERSATION = 'conversation',
	SETTINGS = 'settings',
	GIT = 'git',
	AI = 'ai'
}

/**
 * Represents a command that can be executed from the command palette.
 */
export interface Command {
	/** Unique identifier for the command */
	id: string;

	/** Display name of the command */
	name: string;

	/** Description of what the command does */
	description: string;

	/** Optional icon name (maps to SVG icon) */
	icon?: string;

	/** Category for grouping */
	category: CommandCategory;

	/** Keyboard shortcut (e.g., "Ctrl+Enter") */
	shortcut?: string;

	/** Whether the command is currently available */
	available?: boolean;

	/** Handler function to execute the command */
	handler: () => void | Promise<void>;
}

/**
 * Props for the CommandPalette component.
 */
export interface CommandPaletteProps {
	/** Filter string from user input */
	query: string;

	/** Handler when a command is selected */
	onSelect: (command: Command) => void;

	/** Handler to close the palette */
	onClose: () => void;
}

/**
 * Built-in commands available in the application.
 */
export const BUILT_IN_COMMANDS: Command[] = [
	// Code commands
	{
		id: 'generate',
		name: 'generate',
		description: 'Generate code from description',
		icon: 'code',
		category: CommandCategory.CODE,
		handler: () => {}
	},
	{
		id: 'explain',
		name: 'explain',
		description: 'Explain selected code',
		icon: 'book',
		category: CommandCategory.CODE,
		handler: () => {}
	},
	{
		id: 'refactor',
		name: 'refactor',
		description: 'Refactor selected code',
		icon: 'refresh',
		category: CommandCategory.CODE,
		handler: () => {}
	},
	{
		id: 'optimize',
		name: 'optimize',
		description: 'Optimize code performance',
		icon: 'zap',
		category: CommandCategory.CODE,
		handler: () => {}
	},
	{
		id: 'test',
		name: 'test',
		description: 'Generate tests for code',
		icon: 'check-circle',
		category: CommandCategory.CODE,
		handler: () => {}
	},
	{
		id: 'document',
		name: 'document',
		description: 'Add documentation to code',
		icon: 'file-text',
		category: CommandCategory.CODE,
		handler: () => {}
	},

	// Conversation commands
	{
		id: 'new-chat',
		name: 'new-chat',
		description: 'Start a new conversation',
		icon: 'message-square',
		category: CommandCategory.CONVERSATION,
		shortcut: 'Ctrl+N',
		handler: () => {}
	},
	{
		id: 'clear',
		name: 'clear',
		description: 'Clear current conversation',
		icon: 'trash',
		category: CommandCategory.CONVERSATION,
		handler: () => {}
	},
	{
		id: 'export',
		name: 'export',
		description: 'Export conversation',
		icon: 'download',
		category: CommandCategory.CONVERSATION,
		handler: () => {}
	},

	// Navigation commands
	{
		id: 'goto-file',
		name: 'goto-file',
		description: 'Go to file',
		icon: 'file',
		category: CommandCategory.NAVIGATION,
		shortcut: 'Ctrl+P',
		handler: () => {}
	},
	{
		id: 'goto-symbol',
		name: 'goto-symbol',
		description: 'Go to symbol',
		icon: 'hash',
		category: CommandCategory.NAVIGATION,
		shortcut: 'Ctrl+Shift+O',
		handler: () => {}
	},

	// Git commands
	{
		id: 'git-status',
		name: 'git-status',
		description: 'Show git status',
		icon: 'git-branch',
		category: CommandCategory.GIT,
		handler: () => {}
	},
	{
		id: 'git-commit',
		name: 'git-commit',
		description: 'Create a commit',
		icon: 'git-commit',
		category: CommandCategory.GIT,
		handler: () => {}
	},
	{
		id: 'git-diff',
		name: 'git-diff',
		description: 'Show git diff',
		icon: 'git-compare',
		category: CommandCategory.GIT,
		handler: () => {}
	},

	// AI commands
	{
		id: 'ai-model',
		name: 'ai-model',
		description: 'Change AI model',
		icon: 'cpu',
		category: CommandCategory.AI,
		handler: () => {}
	},
	{
		id: 'ai-clear-context',
		name: 'ai-clear-context',
		description: 'Clear AI context',
		icon: 'x-circle',
		category: CommandCategory.AI,
		handler: () => {}
	},

	// Settings commands
	{
		id: 'settings',
		name: 'settings',
		description: 'Open settings',
		icon: 'settings',
		category: CommandCategory.SETTINGS,
		shortcut: 'Ctrl+,',
		handler: () => {}
	},
	{
		id: 'theme',
		name: 'theme',
		description: 'Toggle theme',
		icon: 'moon',
		category: CommandCategory.SETTINGS,
		handler: () => {}
	}
];

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
export function groupCommandsByCategory(commands: Command[]): Map<CommandCategory, Command[]> {
	const grouped = new Map<CommandCategory, Command[]>();

	for (const cmd of commands) {
		const existing = grouped.get(cmd.category) || [];
		existing.push(cmd);
		grouped.set(cmd.category, existing);
	}

	return grouped;
}

/**
 * Get category display label.
 */
export function getCategoryLabel(category: CommandCategory): string {
	const labels: Record<CommandCategory, string> = {
		[CommandCategory.CODE]: 'Code',
		[CommandCategory.NAVIGATION]: 'Navigation',
		[CommandCategory.CONVERSATION]: 'Conversation',
		[CommandCategory.SETTINGS]: 'Settings',
		[CommandCategory.GIT]: 'Git',
		[CommandCategory.AI]: 'AI'
	};

	return labels[category] || category;
}
