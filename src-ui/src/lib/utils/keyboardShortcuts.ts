/**
 * Keyboard Shortcuts Utility
 *
 * Centralized keyboard shortcut handling for the application.
 * Uses Cmd/Ctrl modifier for cross-platform compatibility.
 *
 * @module keyboardShortcuts
 */

// ============================================================================
// Types
// ============================================================================

/** Keyboard shortcut definition */
export interface ShortcutDefinition {
	/** Key to press (e.g., 'n', 'Enter', 'Escape') */
	key: string;
	/** Whether Ctrl/Cmd is required */
	ctrl?: boolean;
	/** Whether Shift is required */
	shift?: boolean;
	/** Whether Alt is required */
	alt?: boolean;
	/** Handler function */
	handler: (event: KeyboardEvent) => void;
	/** Whether to prevent default behavior */
	preventDefault?: boolean;
	/** Description for help UI */
	description?: string;
}

/** Registered shortcut entry */
interface RegisteredShortcut {
	id: string;
	definition: ShortcutDefinition;
}

// ============================================================================
// State
// ============================================================================

/** Map of registered shortcuts */
const shortcuts = new Map<string, RegisteredShortcut>();

/** Counter for generating unique IDs */
let shortcutCounter = 0;

/** Whether the listener is active */
let listenerActive = false;

// ============================================================================
// Private Functions
// ============================================================================

/**
 * Check if the event matches a shortcut definition.
 */
function matchesShortcut(event: KeyboardEvent, definition: ShortcutDefinition): boolean {
	// Normalize key to lowercase for letter keys
	const eventKey = event.key.toLowerCase();
	const targetKey = definition.key.toLowerCase();

	// Check key match
	if (eventKey !== targetKey) return false;

	// Check modifiers
	const ctrlRequired = definition.ctrl ?? false;
	const shiftRequired = definition.shift ?? false;
	const altRequired = definition.alt ?? false;

	// Use metaKey on Mac, ctrlKey on other platforms
	const isCtrlPressed = event.ctrlKey || event.metaKey;

	if (ctrlRequired !== isCtrlPressed) return false;
	if (shiftRequired !== event.shiftKey) return false;
	if (altRequired !== event.altKey) return false;

	return true;
}

/**
 * Global keyboard event handler.
 */
function handleKeyDown(event: KeyboardEvent): void {
	// Don't trigger shortcuts when typing in input fields
	const target = event.target as HTMLElement;
	const isInputField =
		target.tagName === 'INPUT' ||
		target.tagName === 'TEXTAREA' ||
		target.isContentEditable;

	// Allow Escape and specific shortcuts even in input fields
	const isEscape = event.key === 'Escape';

	for (const { definition } of shortcuts.values()) {
		// Skip shortcuts that require Ctrl when in input field (except Escape)
		if (isInputField && definition.ctrl && !isEscape) continue;

		if (matchesShortcut(event, definition)) {
			if (definition.preventDefault !== false) {
				event.preventDefault();
			}
			definition.handler(event);
			return;
		}
	}
}

/**
 * Start listening for keyboard events.
 */
function startListener(): void {
	if (listenerActive) return;
	listenerActive = true;
	document.addEventListener('keydown', handleKeyDown);
}

/**
 * Stop listening for keyboard events.
 */
function stopListener(): void {
	if (!listenerActive) return;
	listenerActive = false;
	document.removeEventListener('keydown', handleKeyDown);
}

// ============================================================================
// Public API
// ============================================================================

/**
 * Register a keyboard shortcut.
 *
 * @param definition - Shortcut definition
 * @returns Shortcut ID for unregistration
 */
export function registerShortcut(definition: ShortcutDefinition): string {
	const id = `shortcut-${++shortcutCounter}`;
	shortcuts.set(id, { id, definition });

	// Ensure listener is active
	startListener();

	return id;
}

/**
 * Unregister a keyboard shortcut.
 *
 * @param id - Shortcut ID to unregister
 */
export function unregisterShortcut(id: string): void {
	shortcuts.delete(id);

	// Stop listener if no shortcuts remain
	if (shortcuts.size === 0) {
		stopListener();
	}
}

/**
 * Register multiple shortcuts at once.
 *
 * @param definitions - Array of shortcut definitions
 * @returns Array of shortcut IDs
 */
export function registerShortcuts(definitions: ShortcutDefinition[]): string[] {
	return definitions.map((def) => registerShortcut(def));
}

/**
 * Unregister multiple shortcuts.
 *
 * @param ids - Array of shortcut IDs
 */
export function unregisterShortcuts(ids: string[]): void {
	ids.forEach((id) => unregisterShortcut(id));
}

/**
 * Create a shortcut handler that can be easily used in components.
 * Returns cleanup function.
 *
 * @param key - Key to listen for
 * @param handler - Handler function
 * @param options - Additional options
 * @returns Cleanup function
 */
export function useShortcut(
	key: string,
	handler: (event: KeyboardEvent) => void,
	options: {
		ctrl?: boolean;
		shift?: boolean;
		alt?: boolean;
		preventDefault?: boolean;
	} = {}
): () => void {
	const id = registerShortcut({
		key,
		handler,
		ctrl: options.ctrl,
		shift: options.shift,
		alt: options.alt,
		preventDefault: options.preventDefault
	});

	return () => unregisterShortcut(id);
}

/**
 * Get all registered shortcuts (for help UI).
 */
export function getRegisteredShortcuts(): Array<{
	id: string;
	key: string;
	ctrl: boolean;
	shift: boolean;
	alt: boolean;
	description?: string;
}> {
	return Array.from(shortcuts.values()).map(({ id, definition }) => ({
		id,
		key: definition.key,
		ctrl: definition.ctrl ?? false,
		shift: definition.shift ?? false,
		alt: definition.alt ?? false,
		description: definition.description
	}));
}

/**
 * Format a shortcut for display.
 */
export function formatShortcut(shortcut: {
	key: string;
	ctrl?: boolean;
	shift?: boolean;
	alt?: boolean;
}): string {
	const parts: string[] = [];
	const isMac = typeof navigator !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;

	if (shortcut.ctrl) {
		parts.push(isMac ? '⌘' : 'Ctrl');
	}
	if (shortcut.shift) {
		parts.push(isMac ? '⇧' : 'Shift');
	}
	if (shortcut.alt) {
		parts.push(isMac ? '⌥' : 'Alt');
	}

	// Format key
	let key = shortcut.key;
	if (key === ' ') key = 'Space';
	if (key === 'Escape') key = 'Esc';
	if (key === 'ArrowUp') key = '↑';
	if (key === 'ArrowDown') key = '↓';
	if (key === 'ArrowLeft') key = '←';
	if (key === 'ArrowRight') key = '→';
	if (key.length === 1) key = key.toUpperCase();

	parts.push(key);

	return parts.join(isMac ? '' : '+');
}

// ============================================================================
// Common Shortcuts Factory
// ============================================================================

/**
 * Create common application shortcuts.
 */
export function createAppShortcuts(handlers: {
	onNewThread?: () => void;
	onNewConversation?: () => void;
	onFocusInput?: () => void;
	onToggleSidebar?: () => void;
	onEscape?: () => void;
}): string[] {
	const shortcuts: ShortcutDefinition[] = [];

	if (handlers.onNewThread) {
		shortcuts.push({
			key: 'n',
			ctrl: true,
			handler: handlers.onNewThread,
			description: 'New thread'
		});
	}

	if (handlers.onNewConversation) {
		shortcuts.push({
			key: 'n',
			ctrl: true,
			shift: true,
			handler: handlers.onNewConversation,
			description: 'New conversation'
		});
	}

	if (handlers.onFocusInput) {
		shortcuts.push({
			key: '/',
			handler: handlers.onFocusInput,
			description: 'Focus input'
		});
	}

	if (handlers.onToggleSidebar) {
		shortcuts.push({
			key: 'b',
			ctrl: true,
			handler: handlers.onToggleSidebar,
			description: 'Toggle sidebar'
		});
	}

	if (handlers.onEscape) {
		shortcuts.push({
			key: 'Escape',
			handler: handlers.onEscape,
			preventDefault: false,
			description: 'Close / Cancel'
		});
	}

	return registerShortcuts(shortcuts);
}
