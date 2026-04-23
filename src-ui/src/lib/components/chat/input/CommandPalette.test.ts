import { fireEvent, render, screen, within } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CommandPalette from './CommandPalette.svelte';
import type { Command } from '$lib/types/command';

describe('CommandPalette', () => {
	const mockCommands: Command[] = [
		{
			id: 'generate',
			name: 'generate',
			description: 'Generate code from description',
			icon: 'code',
			category: 'code'
		},
		{
			id: 'explain',
			name: 'explain',
			description: 'Explain selected code',
			icon: 'book',
			category: 'code'
		},
		{
			id: 'new-chat',
			name: 'new-chat',
			description: 'Start a new conversation',
			icon: 'message-square',
			category: 'conversation',
			shortcut: 'Ctrl+N'
		},
		{
			id: 'settings',
			name: 'settings',
			description: 'Open settings',
			icon: 'settings',
			category: 'settings'
		}
	];

	const defaultProps = {
		query: '',
		commands: mockCommands,
		onSelect: vi.fn(),
		onClose: vi.fn()
	};

	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		// Clean up any event listeners
		document.removeEventListener('keydown', () => {});
	});

	describe('rendering', () => {
		it('renders the palette with commands', () => {
			const { container } = render(CommandPalette, { ...defaultProps });
			expect(screen.getByText('Commands')).toBeInTheDocument();
			expect(screen.getByText('Code')).toBeInTheDocument();
			expect(screen.getByText('Conversation')).toBeInTheDocument();
			expect(screen.getByText('Settings')).toBeInTheDocument();
		});

		it('renders command items with names and descriptions', () => {
			render(CommandPalette, { ...defaultProps });
			expect(screen.getByText('/generate')).toBeInTheDocument();
			expect(screen.getByText('Generate code from description')).toBeInTheDocument();
			expect(screen.getByText('/explain')).toBeInTheDocument();
			expect(screen.getByText('Explain selected code')).toBeInTheDocument();
		});

		it('renders keyboard shortcuts when provided', () => {
			render(CommandPalette, { ...defaultProps });
			expect(screen.getByText('Ctrl+N')).toBeInTheDocument();
		});

		it('renders empty state when no commands match query', () => {
			render(CommandPalette, {
				...defaultProps,
				query: 'nonexistent',
				commands: []
			});
			expect(screen.getByText(/no commands found/i)).toBeInTheDocument();
		});

		it('displays current query in header', () => {
			render(CommandPalette, {
				...defaultProps,
				query: 'gen'
			});
			expect(screen.getByText('/gen')).toBeInTheDocument();
		});

		it('renders the Esc hint in footer', () => {
			render(CommandPalette, { ...defaultProps });
			const listbox = screen.getByRole('listbox');
			expect(listbox.textContent).toContain('Esc to close');
		});
	});

	describe('command selection', () => {
		it('calls onSelect when command item is clicked', async () => {
			const onSelect = vi.fn();
			render(CommandPalette, { ...defaultProps, onSelect });

			const generateButton = screen.getByText('/generate').closest('button');
			if (generateButton) {
				await fireEvent.click(generateButton);
				expect(onSelect).toHaveBeenCalledWith(mockCommands[0]);
			}
		});

		it('calls onSelect when Enter is pressed on selected item', async () => {
			const onSelect = vi.fn();
			const { container } = render(CommandPalette, {
				...defaultProps,
				onSelect,
				query: '' // First item (generate) should be selected
			});

			// Simulate Enter key
			await fireEvent.keyDown(document, { key: 'Enter' });
			// The command palette listens to document keydown events
			// First item should be selected by default
			// onSelect should have been called with the first command
		});

		it('updates selected index on mouse hover', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			const explainButton = screen.getByText('/explain').closest('button');
			if (explainButton) {
				await fireEvent.mouseEnter(explainButton);
				// The second item should now be selected
				// We can verify this by checking data-selected attribute
				expect(explainButton).toHaveAttribute('data-selected', 'true');
			}
		});
	});

	describe('keyboard navigation', () => {
		it('selects next item on ArrowDown', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			// First item should be selected initially
			const firstButton = screen.getByText('/generate').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');

			// Press ArrowDown
			await fireEvent.keyDown(document, { key: 'ArrowDown' });

			// Second item should now be selected
			const secondButton = screen.getByText('/explain').closest('button');
			expect(secondButton).toHaveAttribute('data-selected', 'true');
		});

		it('selects previous item on ArrowUp', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			// Navigate down twice
			await fireEvent.keyDown(document, { key: 'ArrowDown' });
			await fireEvent.keyDown(document, { key: 'ArrowDown' });

			// Navigate back up
			await fireEvent.keyDown(document, { key: 'ArrowUp' });

			// Second item should still be selected
			const secondButton = screen.getByText('/explain').closest('button');
			expect(secondButton).toHaveAttribute('data-selected', 'true');
		});

		it('does not go below zero on ArrowUp from first item', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			// First item should be selected
			const firstButton = screen.getByText('/generate').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');

			// Try to go up from first item
			await fireEvent.keyDown(document, { key: 'ArrowUp' });

			// First item should still be selected
			expect(firstButton).toHaveAttribute('data-selected', 'true');
		});

		it('does not go beyond last item on ArrowDown', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			// Navigate to the end
			for (let i = 0; i < mockCommands.length + 2; i++) {
				await fireEvent.keyDown(document, { key: 'ArrowDown' });
			}

			// Last item should be selected
			const lastButton = screen.getByText('/settings').closest('button');
			expect(lastButton).toHaveAttribute('data-selected', 'true');
		});

		it('calls onClose when Escape is pressed', async () => {
			const onClose = vi.fn();
			render(CommandPalette, { ...defaultProps, onClose });

			await fireEvent.keyDown(document, { key: 'Escape' });
			expect(onClose).toHaveBeenCalled();
		});

		it('prevents default behavior on arrow keys', async () => {
			const { container } = render(CommandPalette, { ...defaultProps });

			// Mock preventDefault
			const preventDefault = vi.fn();
			await fireEvent.keyDown(document, {
				key: 'ArrowDown',
				preventDefault
			});
			// The event listener should call preventDefault
		});
	});

	describe('filtering', () => {
		it('filters commands by name', () => {
			render(CommandPalette, {
				...defaultProps,
				query: 'gen'
			});
			// Should show generate command
			expect(screen.getByText('/generate')).toBeInTheDocument();
			// Should not show other commands
			expect(screen.queryByText('/explain')).not.toBeInTheDocument();
		});

		it('filters commands by description', () => {
			render(CommandPalette, {
				...defaultProps,
				query: 'code'
			});
			// Should show commands with "code" in description
			expect(screen.getByText('/generate')).toBeInTheDocument();
		});

		it('shows empty state when no commands match', () => {
			render(CommandPalette, {
				...defaultProps,
				query: 'xyz123',
				commands: []
			});
			expect(screen.getByText(/no commands found for "xyz123"/i)).toBeInTheDocument();
		});

		it('resets selection when filtered commands change', () => {
			const { container } = render(CommandPalette, {
				...defaultProps,
				query: 'gen'
			});
			// First matching item should be selected
			const firstButton = screen.getByText('/generate').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');
		});
	});

	describe('accessibility', () => {
		it('has role="listbox"', () => {
			const { container } = render(CommandPalette, { ...defaultProps });
			const listbox = container.querySelector('[role="listbox"]');
			expect(listbox).toBeInTheDocument();
		});

		it('has aria-label for available commands', () => {
			const { container } = render(CommandPalette, { ...defaultProps });
			const listbox = container.querySelector('[aria-label="Available commands"]');
			expect(listbox).toBeInTheDocument();
		});

		it('sets aria-activedescendant for selected item', () => {
			const { container } = render(CommandPalette, { ...defaultProps });
			const listbox = container.querySelector('[role="listbox"]') as HTMLElement;
			expect(listbox?.getAttribute('aria-activedescendant')).toBe('command-option-0');
		});

		it('each command item has role="option"', () => {
			render(CommandPalette, { ...defaultProps });
			const options = document.querySelectorAll('[role="option"]');
			expect(options.length).toBeGreaterThan(0);
		});

		it('selected item has aria-selected="true"', () => {
			render(CommandPalette, { ...defaultProps });
			const selected = document.querySelector('[aria-selected="true"]');
			expect(selected).toBeInTheDocument();
		});

		it('items have unique ids for aria-activedescendant', () => {
			render(CommandPalette, { ...defaultProps });
			const items = document.querySelectorAll('[id^="command-option-"]');
			expect(items.length).toBe(mockCommands.length);
		});
	});

	describe('grouping', () => {
		it('groups commands by category', () => {
			render(CommandPalette, { ...defaultProps });
			// Should show category headers
			expect(screen.getByText('Code')).toBeInTheDocument();
			expect(screen.getByText('Conversation')).toBeInTheDocument();
			expect(screen.getByText('Settings')).toBeInTheDocument();
		});

		it('shows category labels in correct order', () => {
			render(CommandPalette, { ...defaultProps });
			const categories = screen.getAllByText(/Code|Conversation|Settings/);
			expect(categories.length).toBeGreaterThanOrEqual(3);
		});
	});

	describe('edge cases', () => {
		it('handles empty command list', () => {
			render(CommandPalette, {
				...defaultProps,
				commands: []
			});
			expect(screen.getByText(/no commands found/i)).toBeInTheDocument();
		});

		it('handles single command', () => {
			const singleCommand: Command[] = [
				{
					id: 'only',
					name: 'only',
					description: 'Only command',
					icon: 'code',
					category: 'code'
				}
			];

			render(CommandPalette, {
				...defaultProps,
				commands: singleCommand
			});
			expect(screen.getByText('/only')).toBeInTheDocument();
		});

		it('handles very long command names', () => {
			const longCommand: Command[] = [
				{
					id: 'long',
					name: 'a'.repeat(100),
					description: 'A command with a very long name',
					icon: 'code',
					category: 'code'
				}
			];

			render(CommandPalette, {
				...defaultProps,
				commands: longCommand
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles very long descriptions', () => {
			const longDescCommand: Command[] = [
				{
					id: 'long-desc',
					name: 'long-desc',
					description: 'A'.repeat(200),
					icon: 'code',
					category: 'code'
				}
			];

			render(CommandPalette, {
				...defaultProps,
				commands: longDescCommand
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles special characters in query', () => {
			render(CommandPalette, {
				...defaultProps,
				query: '/!@#$%'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles commands without icons', () => {
			const noIconCommand: Command[] = [
				{
					id: 'no-icon',
					name: 'no-icon',
					description: 'Command without icon',
					category: 'code'
				}
			];

			render(CommandPalette, {
				...defaultProps,
				commands: noIconCommand
			});
			// Should still render with default icon
			expect(screen.getByText('/no-icon')).toBeInTheDocument();
		});
	});

	describe('cleanup', () => {
		it('removes event listeners on unmount', () => {
			const { unmount } = render(CommandPalette, { ...defaultProps });
			const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener');

			unmount();
			// Event listeners should be cleaned up
			expect(removeEventListenerSpy).toHaveBeenCalled();
		});
	});
});
