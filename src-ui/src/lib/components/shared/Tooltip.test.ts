import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import Tooltip from './Tooltip.svelte';

describe('Tooltip', () => {
	let triggerElement: HTMLElement;

	beforeEach(() => {
		// Create a trigger element wrapper for testing
		triggerElement = document.createElement('div');
		document.body.appendChild(triggerElement);
	});

	afterEach(() => {
		document.body.innerHTML = '';
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders children content', () => {
			render(Tooltip, {
				content: 'Tooltip text',
				children: 'Trigger'
			});

			expect(screen.getByText('Trigger')).toBeInTheDocument();
		});

		it('does not render tooltip when initially hidden', () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				children: 'Trigger'
			});

			const tooltip = container.querySelector('.tooltip');
			expect(tooltip).not.toHaveClass('visible');
		});

		it('renders string content', () => {
			render(Tooltip, {
				content: 'Tooltip text',
				children: 'Trigger'
			});

			expect(screen.getByText('Tooltip text')).toBeInTheDocument();
		});
	});

	describe('showing and hiding', () => {
		it('shows tooltip on mouse enter', async () => {
			render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				// Tooltip uses position: fixed, so it's in document.body, not container
				const tooltip = document.body.querySelector('[class*="tooltip"]');
				expect(tooltip?.classList.contains('visible')).toBe(true);
			});
		});

		it('hides tooltip on mouse leave', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				hideDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('visible');
			});

			await fireEvent.mouseLeave(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).not.toHaveClass('visible');
			});
		});

		it('shows tooltip on focus', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.focusIn(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('visible');
			});
		});

		it('hides tooltip on focus out', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				hideDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.focusIn(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('visible');
			});

			await fireEvent.focusOut(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).not.toHaveClass('visible');
			});
		});

		it('hides tooltip on Escape key', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('visible');
			});

			await fireEvent.keyDown(document, { key: 'Escape' });

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).not.toHaveClass('visible');
			});
		});
	});

	describe('delays', () => {
		it('respects show delay', async () => {
			const showDelay = 200;
			const startTime = Date.now();

			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(
				() => {
					const tooltip = container.querySelector('.tooltip');
					expect(tooltip).toHaveClass('visible');
				},
				{ timeout: 500 }
			);

			const elapsed = Date.now() - startTime;
			expect(elapsed).toBeGreaterThanOrEqual(showDelay - 10); // Allow small margin
		});

		it('respects hide delay', async () => {
			const hideDelay = 100;
			const startTime = Date.now();

			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				hideDelay,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('visible');
			});

			await fireEvent.mouseLeave(trigger);

			await waitFor(
				() => {
					const tooltip = container.querySelector('.tooltip');
					expect(tooltip).not.toHaveClass('visible');
				},
				{ timeout: 300 }
			);

			const elapsed = Date.now() - startTime;
			expect(elapsed).toBeGreaterThanOrEqual(hideDelay - 10); // Allow small margin
		});
	});

	describe('disabled state', () => {
		it('does not show when disabled', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				disabled: true,
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			// Wait a bit to ensure it doesn't appear
			await new Promise((resolve) => setTimeout(resolve, 100));

			const tooltip = container.querySelector('.tooltip');
			expect(tooltip).not.toHaveClass('visible');
		});
	});

	describe('positions', () => {
		it('positions tooltip on top by default', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('top');
			});
		});

		it('positions tooltip on bottom', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				position: 'bottom',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('bottom');
			});
		});

		it('positions tooltip on left', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				position: 'left',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('left');
			});
		});

		it('positions tooltip on right', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				position: 'right',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('right');
			});
		});
	});

	describe('styling', () => {
		it('respects maxWidth prop', async () => {
			const maxWidth = '300px';
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				maxWidth,
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip?.style.maxWidth).toBe(maxWidth);
			});
		});

		it('has fixed position', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip?.style.position).toBe('fixed');
			});
		});
	});

	describe('accessibility', () => {
		it('has role="tooltip"', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('[role="tooltip"]');
				expect(tooltip).toBeInTheDocument();
			});
		});

		it('has aria-hidden when not visible', () => {
			render(Tooltip, {
				content: 'Tooltip text',
				children: 'Trigger'
			});

			const tooltip = screen.queryByRole('tooltip');
			if (tooltip) {
				expect(tooltip).toHaveAttribute('aria-hidden', 'true');
			}
		});
	});

	describe('arrow', () => {
		it('renders arrow element', async () => {
			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const arrow = container.querySelector('.arrow');
				expect(arrow).toBeInTheDocument();
			});
		});
	});

	describe('reduced motion', () => {
		it('applies no animation class when prefers reduced motion', async () => {
			// Mock matchMedia for reduced motion
			vi.spyOn(window, 'matchMedia').mockReturnValue({
				matches: true,
				media: '(prefers-reduced-motion: reduce)',
				addListener: vi.fn(),
				removeListener: vi.fn(),
				addEventListener: vi.fn(),
				removeEventListener: vi.fn(),
				dispatchEvent: vi.fn()
			} as unknown as MediaQueryList);

			const { container } = render(Tooltip, {
				content: 'Tooltip text',
				showDelay: 0,
				children: 'Trigger'
			});

			const trigger = screen.getByText('Trigger');
			await fireEvent.mouseEnter(trigger);

			await waitFor(() => {
				const tooltip = container.querySelector('.tooltip');
				expect(tooltip).toHaveClass('noAnimation');
			});
		});
	});
});
