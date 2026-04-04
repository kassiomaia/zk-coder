import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Popover from './Popover.svelte';
import type { PopoverItem, PopoverOrigin } from './popover.types';

describe('Popover', () => {
	let anchorEl: HTMLElement;
	let mockItems: PopoverItem[];
	let mockOnSelect: ReturnType<typeof vi.fn>;
	let mockOnClose: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		// Create a mock anchor element
		anchorEl = document.createElement('button');
		anchorEl.textContent = 'Trigger';
		document.body.appendChild(anchorEl);

		mockItems = [
			{ id: '1', label: 'First Item' },
			{ id: '2', label: 'Second Item' },
			{ id: '3', label: 'Third Item' }
		];

		mockOnSelect = vi.fn();
		mockOnClose = vi.fn();
		vi.clearAllMocks();
	});

	afterEach(() => {
		document.body.innerHTML = '';
	});

	describe('rendering', () => {
		it('does not render when open is false', () => {
			const { container } = render(Popover, {
				open: false,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			expect(container.querySelector('.popover')).not.toBeInTheDocument();
		});

		it('renders when open is true', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const popover = container.querySelector('.popover');
			expect(popover).toBeInTheDocument();
		});

		it('renders all menu items', () => {
			render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			expect(screen.getByText('First Item')).toBeInTheDocument();
			expect(screen.getByText('Second Item')).toBeInTheDocument();
			expect(screen.getByText('Third Item')).toBeInTheDocument();
		});

		it('has role="menu"', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const popover = container.querySelector('[role="menu"]');
			expect(popover).toBeInTheDocument();
		});
	});

	describe('item interactions', () => {
		it('calls onSelect and onClose when item is clicked', async () => {
			render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const firstItem = screen.getByText('First Item');
			await fireEvent.click(firstItem);

			expect(mockOnSelect).toHaveBeenCalledWith(mockItems[0]);
			expect(mockOnClose).toHaveBeenCalledTimes(1);
		});

		it('does not call onSelect for disabled items', async () => {
			const itemsWithDisabled = [...mockItems, { id: '4', label: 'Disabled Item', disabled: true }];

			render(Popover, {
				open: true,
				items: itemsWithDisabled,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const disabledItem = screen.getByText('Disabled Item');
			await fireEvent.click(disabledItem);

			expect(mockOnSelect).not.toHaveBeenCalled();
		});

		it('disables button for disabled items', () => {
			const itemsWithDisabled = [{ id: '1', label: 'Disabled Item', disabled: true }];

			render(Popover, {
				open: true,
				items: itemsWithDisabled,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const button = screen.getByRole('menuitem');
			expect(button).toBeDisabled();
		});
	});

	describe('keyboard interaction', () => {
		it('closes on Escape key', async () => {
			render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			await fireEvent.keyDown(document, { key: 'Escape' });

			expect(mockOnClose).toHaveBeenCalledTimes(1);
		});

		it('does not close on Escape when not open', async () => {
			render(Popover, {
				open: false,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			await fireEvent.keyDown(document, { key: 'Escape' });

			expect(mockOnClose).not.toHaveBeenCalled();
		});
	});

	describe('origins', () => {
		it('applies top-left origin class', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose,
				origin: 'top-left' as PopoverOrigin
			});

			const popover = container.querySelector('.popover');
			expect(popover).toHaveClass('originTopLeft');
		});

		it('applies top-right origin class', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose,
				origin: 'top-right' as PopoverOrigin
			});

			const popover = container.querySelector('.popover');
			expect(popover).toHaveClass('originTopRight');
		});

		it('applies bottom-left origin class', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose,
				origin: 'bottom-left' as PopoverOrigin
			});

			const popover = container.querySelector('.popover');
			expect(popover).toHaveClass('originBottomLeft');
		});

		it('applies bottom-right origin class', () => {
			const { container } = render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose,
				origin: 'bottom-right' as PopoverOrigin
			});

			const popover = container.querySelector('.popover');
			expect(popover).toHaveClass('originBottomRight');
		});
	});

	describe('item features', () => {
		it('renders item with icon', () => {
			const iconHtml = '<svg data-testid="test-icon"></svg>';

			render(Popover, {
				open: true,
				items: [{ id: '1', label: 'Item with Icon', iconHtml }],
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const icon = document.querySelector('[data-testid="test-icon"]');
			expect(icon).toBeInTheDocument();
		});

		it('renders item with shortcut', () => {
			render(Popover, {
				open: true,
				items: [{ id: '1', label: 'Save', shortcut: '⌘S' }],
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			expect(screen.getByText('⌘S')).toBeInTheDocument();
		});

		it('renders item with all features', () => {
			const iconHtml = '<svg data-testid="icon"></svg>';

			render(Popover, {
				open: true,
				items: [{ id: '1', label: 'Full Item', iconHtml, shortcut: '⌘K' }],
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			expect(screen.getByText('Full Item')).toBeInTheDocument();
			expect(screen.getByText('⌘K')).toBeInTheDocument();
			expect(document.querySelector('[data-testid="icon"]')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="menuitem" for each item', () => {
			render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const items = screen.getAllByRole('menuitem');
			expect(items).toHaveLength(3);
		});

		it('has type="button" for each item', () => {
			render(Popover, {
				open: true,
				items: mockItems,
				onSelect: mockOnSelect,
				onClose: mockOnClose
			});

			const items = screen.getAllByRole('menuitem');
			items.forEach((item) => {
				expect(item).toHaveAttribute('type', 'button');
			});
		});
	});
});
