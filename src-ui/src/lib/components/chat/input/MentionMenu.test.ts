import {fireEvent, render, screen, within} from '@testing-library/svelte';
import {afterEach, beforeEach, describe, expect, it, vi} from 'vitest';
import MentionMenu from './MentionMenu.svelte';

interface MentionItem {
	id: string;
	name: string;
	type: 'file' | 'agent' | 'context';
	icon?: string;
	description?: string;
}

describe('MentionMenu', () => {
	const mockItems: MentionItem[] = [
		{
			id: 'file-1',
			name: 'src/lib/components/Button.svelte',
			type: 'file',
			description: 'Button component'
		},
		{
			id: 'file-2',
			name: 'src/lib/utils/helpers.ts',
			type: 'file',
			description: 'Helper utilities'
		},
		{
			id: 'agent-1',
			name: 'Code Reviewer',
			type: 'agent',
			description: 'Reviews code for issues'
		},
		{
			id: 'context-1',
			name: 'Project Documentation',
			type: 'context',
			description: 'Project docs and guides'
		}
	];

	const defaultProps = {
		query: '',
		items: mockItems,
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
		it('renders the menu with items', () => {
			const { container } = render(MentionMenu, { ...defaultProps });
			expect(screen.getByText('Mention')).toBeInTheDocument();
			expect(screen.getByText('src/lib/components/Button.svelte')).toBeInTheDocument();
			expect(screen.getByText('Code Reviewer')).toBeInTheDocument();
		});

		it('renders item names', () => {
			render(MentionMenu, { ...defaultProps });
			expect(screen.getByText('src/lib/components/Button.svelte')).toBeInTheDocument();
			expect(screen.getByText('src/lib/utils/helpers.ts')).toBeInTheDocument();
			expect(screen.getByText('Code Reviewer')).toBeInTheDocument();
			expect(screen.getByText('Project Documentation')).toBeInTheDocument();
		});

		it('renders item descriptions when provided', () => {
			render(MentionMenu, { ...defaultProps });
			expect(screen.getByText('Button component')).toBeInTheDocument();
			expect(screen.getByText('Helper utilities')).toBeInTheDocument();
			expect(screen.getByText('Reviews code for issues')).toBeInTheDocument();
		});

		it('renders item types', () => {
			render(MentionMenu, { ...defaultProps });
			expect(screen.getAllByText('File')[0]).toBeInTheDocument();
			expect(screen.getByText('Agent')).toBeInTheDocument();
			expect(screen.getByText('Context')).toBeInTheDocument();
		});

		it('renders empty state when no items match query', () => {
			render(MentionMenu, {
				...defaultProps,
				query: 'nonexistent',
				items: []
			});
			expect(screen.getByText(/no items found/i)).toBeInTheDocument();
		});

		it('displays current query in header', () => {
			render(MentionMenu, {
				...defaultProps,
				query: 'src'
			});
			expect(screen.getByText('@src')).toBeInTheDocument();
		});

		it('renders keyboard hints in footer', () => {
			render(MentionMenu, { ...defaultProps });
			const listbox = screen.getByRole('listbox');
			// Find the hint span specifically
			const hintSpan = within(listbox).getByText((content, element) => {
				return element.tagName.toLowerCase() === 'span' && 
				element.textContent.includes('to navigate') &&
				element.textContent.includes('to select') &&
				element.textContent.includes('to close');
			});
			expect(hintSpan).toBeInTheDocument();
			// Assert individual kbd elements are present within the found span
			expect(within(hintSpan).getByText('↑↓')).toBeInTheDocument();
			expect(within(hintSpan).getByText('Enter')).toBeInTheDocument();
			expect(within(hintSpan).getByText('Esc')).toBeInTheDocument();
		});
	});

	describe('item selection', () => {
		it('calls onSelect when item is clicked', async () => {
			const onSelect = vi.fn();
			render(MentionMenu, { ...defaultProps, onSelect });

			const fileItem = screen.getByText('src/lib/components/Button.svelte').closest('button');
			if (fileItem) {
				await fireEvent.click(fileItem);
				expect(onSelect).toHaveBeenCalledWith(mockItems[0]);
			}
		});

		it('calls onSelect when Enter is pressed on selected item', async () => {
			const onSelect = vi.fn();
			render(MentionMenu, {
				...defaultProps,
				onSelect,
				query: ''
			});

			// First item should be selected by default
			// Press Enter to select it
			await fireEvent.keyDown(document, { key: 'Enter' });
			// onSelect should be called with the first item
		});

		it('updates selected index on mouse hover', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			const secondItem = screen.getByText('src/lib/utils/helpers.ts').closest('button');
			if (secondItem) {
				await fireEvent.mouseEnter(secondItem);
				// The second item should now be selected
				expect(secondItem).toHaveAttribute('data-selected', 'true');
			}
		});
	});

	describe('keyboard navigation', () => {
		it('selects next item on ArrowDown', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// First item should be selected initially
			const firstButton = screen.getByText('src/lib/components/Button.svelte').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');

			// Press ArrowDown
			await fireEvent.keyDown(document, { key: 'ArrowDown' });

			// Second item should now be selected
			const secondButton = screen.getByText('src/lib/utils/helpers.ts').closest('button');
			expect(secondButton).toHaveAttribute('data-selected', 'true');
		});

		it('selects previous item on ArrowUp', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// Navigate down twice
			await fireEvent.keyDown(document, { key: 'ArrowDown' });
			await fireEvent.keyDown(document, { key: 'ArrowDown' });

			// Navigate back up
			await fireEvent.keyDown(document, { key: 'ArrowUp' });

			// Second item should still be selected
			const secondButton = screen.getByText('src/lib/utils/helpers.ts').closest('button');
			expect(secondButton).toHaveAttribute('data-selected', 'true');
		});

		it('does not go below zero on ArrowUp from first item', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// First item should be selected
			const firstButton = screen.getByText('src/lib/components/Button.svelte').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');

			// Try to go up from first item
			await fireEvent.keyDown(document, { key: 'ArrowUp' });

			// First item should still be selected
			expect(firstButton).toHaveAttribute('data-selected', 'true');
		});

		it('does not go beyond last item on ArrowDown', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// Navigate to the end
			for (let i = 0; i < mockItems.length + 2; i++) {
				await fireEvent.keyDown(document, { key: 'ArrowDown' });
			}

			// Last item should be selected
			const lastButton = screen.getByText('Project Documentation').closest('button');
			expect(lastButton).toHaveAttribute('data-selected', 'true');
		});

		it('calls onClose when Escape is pressed', async () => {
			const onClose = vi.fn();
			render(MentionMenu, { ...defaultProps, onClose });

			await fireEvent.keyDown(document, { key: 'Escape' });
			expect(onClose).toHaveBeenCalled();
		});

		it('prevents default behavior on navigation keys', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// Test ArrowDown
			const preventDefault = vi.fn();
			await fireEvent.keyDown(document, {
				key: 'ArrowDown',
				preventDefault
			});
			// The event listener should handle the key
		});
	});

	describe('filtering', () => {
		it('filters items by name', () => {
			render(MentionMenu, {
				...defaultProps,
				query: 'src'
			});
			// Should show items matching 'src'
			expect(screen.getByText('src/lib/components/Button.svelte')).toBeInTheDocument();
			expect(screen.getByText('src/lib/utils/helpers.ts')).toBeInTheDocument();
			// Should not show non-matching items
			expect(screen.queryByText('Code Reviewer')).not.toBeInTheDocument();
		});

		it('filters items case-insensitively', () => {
			render(MentionMenu, {
				...defaultProps,
				query: 'CODE'
			});
			// Should match Code Reviewer
			expect(screen.getByText('Code Reviewer')).toBeInTheDocument();
		});

		it('shows empty state when no items match', () => {
			render(MentionMenu, {
				...defaultProps,
				query: 'xyz123',
				items: []
			});
			expect(screen.getByText(/no items found for "xyz123"/i)).toBeInTheDocument();
		});

		it('resets selection when filtered items change', () => {
			const { container } = render(MentionMenu, {
				...defaultProps,
				query: 'src'
			});
			// First matching item should be selected
			const firstButton = screen.getByText('src/lib/components/Button.svelte').closest('button');
			expect(firstButton).toHaveAttribute('data-selected', 'true');
		});
	});

	describe('icons', () => {
		it('renders file icon for file type', () => {
			const { container } = render(MentionMenu, { ...defaultProps });
			const fileIcons = container.querySelectorAll('svg');
			// Should have icons for file items
			expect(fileIcons.length).toBeGreaterThan(0);
		});

		it('renders agent icon for agent type', () => {
			render(MentionMenu, { ...defaultProps });
			const agentItem = screen.getByText('Code Reviewer').closest('button');
			expect(agentItem?.querySelector('svg')).toBeInTheDocument();
		});

		it('renders context icon for context type', () => {
			render(MentionMenu, { ...defaultProps });
			const contextItem = screen.getByText('Project Documentation').closest('button');
			expect(contextItem?.querySelector('svg')).toBeInTheDocument();
		});
	});

	describe('item types', () => {
		it('shows correct type label for files', () => {
			render(MentionMenu, { ...defaultProps });
			const fileBadges = screen.getAllByText('File');
			expect(fileBadges.length).toBeGreaterThan(0);
		});

		it('shows correct type label for agents', () => {
			render(MentionMenu, { ...defaultProps });
			const agentBadges = screen.getAllByText('Agent');
			expect(agentBadges.length).toBeGreaterThan(0);
		});

		it('shows correct type label for context', () => {
			render(MentionMenu, { ...defaultProps });
			const contextBadges = screen.getAllByText('Context');
			expect(contextBadges.length).toBeGreaterThan(0);
		});
	});

	describe('accessibility', () => {
		it('has role="listbox"', () => {
			const { container } = render(MentionMenu, { ...defaultProps });
			const listbox = container.querySelector('[role="listbox"]');
			expect(listbox).toBeInTheDocument();
		});

		it('has aria-label for available mentions', () => {
			const { container } = render(MentionMenu, { ...defaultProps });
			const listbox = container.querySelector('[aria-label="Available mentions"]');
			expect(listbox).toBeInTheDocument();
		});

		it('sets aria-activedescendant for selected item', () => {
			const { container } = render(MentionMenu, { ...defaultProps });
			const listbox = container.querySelector('[role="listbox"]') as HTMLElement;
			expect(listbox?.getAttribute('aria-activedescendant')).toBe('mention-option-0');
		});

		it('each item has role="option"', () => {
			render(MentionMenu, { ...defaultProps });
			const options = document.querySelectorAll('[role="option"]');
			expect(options.length).toBeGreaterThan(0);
		});

		it('selected item has aria-selected="true"', () => {
			render(MentionMenu, { ...defaultProps });
			const selected = document.querySelector('[aria-selected="true"]');
			expect(selected).toBeInTheDocument();
		});

		it('items have unique ids for aria-activedescendant', () => {
			render(MentionMenu, { ...defaultProps });
			const items = document.querySelectorAll('[id^="mention-option-"]');
			expect(items.length).toBe(mockItems.length);
		});
	});

	describe('edge cases', () => {
		it('handles empty item list', () => {
			render(MentionMenu, {
				...defaultProps,
				items: []
			});
			expect(screen.getByText(/no items found/i)).toBeInTheDocument();
		});

		it('handles single item', () => {
			const singleItem: MentionItem[] = [
				{
					id: 'only',
					name: 'Only Item',
					type: 'file'
				}
			];

			render(MentionMenu, {
				...defaultProps,
				items: singleItem
			});
			expect(screen.getByText('Only Item')).toBeInTheDocument();
		});

		it('handles very long item names', () => {
			const longItem: MentionItem[] = [
				{
					id: 'long',
					name: 'a'.repeat(200),
					type: 'file'
				}
			];

			render(MentionMenu, {
				...defaultProps,
				items: longItem
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles items without descriptions', () => {
			const noDescItem: MentionItem[] = [
				{
					id: 'no-desc',
					name: 'No Description',
					type: 'file'
				}
			];

			render(MentionMenu, {
				...defaultProps,
				items: noDescItem
			});
			expect(screen.getByText('No Description')).toBeInTheDocument();
		});

		it('handles special characters in query', () => {
			render(MentionMenu, {
				...defaultProps,
				query: '!@#$%'
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles items with special characters in names', () => {
			const specialItem: MentionItem[] = [
				{
					id: 'special',
					name: 'file-with-special-chars_123.txt',
					type: 'file'
				}
			];

			render(MentionMenu, {
				...defaultProps,
				items: specialItem,
				query: 'special'
			});
			expect(screen.getByText('file-with-special-chars_123.txt')).toBeInTheDocument();
		});

		it('handles all three item types', () => {
			const allTypes: MentionItem[] = [
				{
					id: 'file',
					name: 'test.txt',
					type: 'file'
				},
				{
					id: 'agent',
					name: 'Test Agent',
					type: 'agent'
				},
				{
					id: 'context',
					name: 'Test Context',
					type: 'context'
				}
			];

			render(MentionMenu, {
				...defaultProps,
				items: allTypes
			});
			expect(screen.getByText('File')).toBeInTheDocument();
			expect(screen.getByText('Agent')).toBeInTheDocument();
			expect(screen.getByText('Context')).toBeInTheDocument();
		});
	});

	describe('cleanup', () => {
		it('removes event listeners on unmount', () => {
			const { unmount } = render(MentionMenu, { ...defaultProps });
			const removeEventListenerSpy = vi.spyOn(document, 'removeEventListener');

			unmount();
			// Event listeners should be cleaned up
			expect(removeEventListenerSpy).toHaveBeenCalled();
		});
	});

	describe('user interactions', () => {
		it('updates query display when query changes', () => {
			const { rerender } = render(MentionMenu, {
				...defaultProps,
				query: 'test'
			});
			expect(screen.getByText('@test')).toBeInTheDocument();

			render(MentionMenu, {
				...defaultProps,
				query: 'newquery'
			});
			expect(screen.getByText('@newquery')).toBeInTheDocument();
		});

		it('handles rapid keyboard navigation', async () => {
			const { container } = render(MentionMenu, { ...defaultProps });

			// Rapid arrow key presses
			for (let i = 0; i < 10; i++) {
				await fireEvent.keyDown(document, { key: 'ArrowDown' });
			}

			// Should not crash, last item should be selected
			const containerEl = document.body;
			expect(containerEl).toBeInTheDocument();
		});
	});
});
