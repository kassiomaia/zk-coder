import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import IconButton from './IconButton.svelte';

describe('IconButton', () => {
	// Create a simple icon snippet
	const iconSnippet = () => `
		<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<circle cx="11" cy="11" r="8"/>
			<path d="m21 21-4.35-4.35"/>
		</svg>
	`;

	describe('rendering', () => {
		it('renders button element', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Search'
			});

			const button = screen.getByRole('button');
			expect(button).toBeInTheDocument();
		});

		it('has accessible label', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Search'
			});

			const button = screen.getByRole('button', { name: 'Search' });
			expect(button).toBeInTheDocument();
		});

		it('renders icon content', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Search'
			});

			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});
	});

	describe('variants', () => {
		it('applies ghost variant by default', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('ghost');
		});

		it('applies subtle variant', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				variant: 'subtle'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('subtle');
		});

		it('applies solid variant', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				variant: 'solid'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('solid');
		});

		it('applies transparent variant', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				variant: 'transparent'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('transparent');
		});
	});

	describe('intents', () => {
		it('applies default intent', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				intent: 'default'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('default');
		});

		it('applies success intent', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				intent: 'success'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('success');
		});

		it('applies warning intent', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				intent: 'warning'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('warning');
		});

		it('applies danger intent', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				intent: 'danger'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('danger');
		});
	});

	describe('sizes', () => {
		it('applies medium size by default', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('md');
		});

		it('applies extra small size', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				size: 'xs'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('xs');
		});

		it('applies small size', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				size: 'sm'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('sm');
		});

		it('applies large size', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				size: 'lg'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('lg');
		});
	});

	describe('disabled state', () => {
		it('is enabled by default', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test'
			});

			const button = screen.getByRole('button');
			expect(button).not.toBeDisabled();
		});

		it('is disabled when disabled prop is true', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				disabled: true
			});

			const button = screen.getByRole('button');
			expect(button).toBeDisabled();
		});

		it('has aria-disabled when disabled', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				disabled: true
			});

			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-disabled', 'true');
		});

		it('does not call onclick when disabled', async () => {
			const handleClick = vi.fn();
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				disabled: true,
				onclick: handleClick
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).not.toHaveBeenCalled();
		});
	});

	describe('loading state', () => {
		it('shows spinner when loading', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				loading: true
			});

			const spinner = container.querySelector('.spinner');
			expect(spinner).toBeInTheDocument();
		});

		it('has aria-busy when loading', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				loading: true
			});

			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-busy', 'true');
		});

		it('does not call onclick when loading', async () => {
			const handleClick = vi.fn();
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				loading: true,
				onclick: handleClick
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).not.toHaveBeenCalled();
		});
	});

	describe('active state', () => {
		it('applies active class when active', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				active: true
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('active');
		});

		it('has aria-pressed when active', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				active: true
			});

			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-pressed', 'true');
		});
	});

	describe('click interactions', () => {
		it('calls onclick handler when clicked', async () => {
			const handleClick = vi.fn();
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				onclick: handleClick
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).toHaveBeenCalledTimes(1);
		});

		it('passes event to onclick handler', async () => {
			const handleClick = vi.fn();
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				onclick: handleClick
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).toHaveBeenCalledWith(expect.any(MouseEvent));
		});
	});

	describe('tooltip', () => {
		it('shows tooltip by default when not disabled', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Search',
				showTooltip: true
			});

			// Tooltip should be rendered (may not be visible without hover)
			const tooltipText = screen.queryByText('Search');
			expect(tooltipText).toBeInTheDocument();
		});

		it('uses custom tooltip when provided', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Search',
				tooltip: 'Click to search'
			});

			const tooltipText = screen.queryByText('Click to search');
			expect(tooltipText).toBeInTheDocument();
		});

		it('does not show tooltip when disabled', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Search',
				disabled: true,
				showTooltip: true
			});

			// When disabled, tooltip should not be rendered
			const tooltipWrapper = document.querySelector('[data-tooltip]');
			expect(tooltipWrapper).not.toBeInTheDocument();
		});

		it('respects tooltip position', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				tooltipPosition: 'bottom'
			});

			const button = screen.getByRole('button');
			expect(button).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has aria-label from label prop', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Close dialog'
			});

			const button = screen.getByRole('button', { name: 'Close dialog' });
			expect(button).toBeInTheDocument();
		});

		it('has correct type attribute', () => {
			render(IconButton, {
				icon: iconSnippet,
				label: 'Submit',
				type: 'submit'
			});

			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('type', 'submit');
		});
	});

	describe('custom class', () => {
		it('applies custom className', () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test',
				class: 'my-custom-class'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('my-custom-class');
		});
	});

	describe('pressed state interaction', () => {
		it('handles mouse press state', async () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test'
			});

			const button = container.querySelector('button');
			await fireEvent.mouseDown(button!);
			await fireEvent.mouseUp(button!);

			expect(button).toBeInTheDocument();
		});

		it('handles mouse leave', async () => {
			const { container } = render(IconButton, {
				icon: iconSnippet,
				label: 'Test'
			});

			const button = container.querySelector('button');
			await fireEvent.mouseLeave(button!);

			expect(button).toBeInTheDocument();
		});
	});
});
