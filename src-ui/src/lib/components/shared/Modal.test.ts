import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Modal from './Modal.svelte';

describe('Modal', () => {
	beforeEach(() => {
		// Reset body styles before each test
		document.body.style.overflow = '';
	});

	afterEach(() => {
		// Clean up body styles after each test
		document.body.style.overflow = '';
	});

	describe('rendering', () => {
		it('does not render when open is false', () => {
			const { container } = render(Modal, {
				open: false,
				onclose: vi.fn()
			});

			expect(container.querySelector('[role="dialog"]')).not.toBeInTheDocument();
			expect(container.querySelector('.modal-overlay')).not.toBeInTheDocument();
		});

		it('renders when open is true', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				title: 'Test Modal'
			});

			expect(screen.getByRole('dialog')).toBeInTheDocument();
			expect(screen.getByText('Test Modal')).toBeInTheDocument();
		});

		it('renders title when provided', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				title: 'My Modal Title'
			});

			expect(screen.getByText('My Modal Title')).toBeInTheDocument();
		});

		it('renders children content', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				children: 'Modal content goes here'
			});

			expect(screen.getByText('Modal content goes here')).toBeInTheDocument();
		});
	});

	describe('sizes', () => {
		it('applies default size', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn()
			});
			const modal = container.querySelector('.modal');
			expect(modal).toHaveClass('default');
		});

		it('applies large size', () => {
			const { container } = render(Modal, {
				open: true,
				size: 'lg',
				onclose: vi.fn()
			});
			const modal = container.querySelector('.modal');
			expect(modal).toHaveClass('lg');
		});

		it('applies extra large size', () => {
			const { container } = render(Modal, {
				open: true,
				size: 'xl',
				onclose: vi.fn()
			});
			const modal = container.querySelector('.modal');
			expect(modal).toHaveClass('xl');
		});

		it('applies full size', () => {
			const { container } = render(Modal, {
				open: true,
				size: 'full',
				onclose: vi.fn()
			});
			const modal = container.querySelector('.modal');
			expect(modal).toHaveClass('full');
		});
	});

	describe('close button', () => {
		it('shows close button by default', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				title: 'Test'
			});

			const closeButton = screen.getByLabelText(/Close/i);
			expect(closeButton).toBeInTheDocument();
		});

		it('hides close button when showClose is false', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				title: 'Test',
				showClose: false
			});

			const closeButton = screen.queryByLabelText(/Close/i);
			expect(closeButton).not.toBeInTheDocument();
		});

		it('calls onclose when close button is clicked', async () => {
			const onclose = vi.fn();
			render(Modal, {
				open: true,
				onclose,
				title: 'Test'
			});

			const closeButton = screen.getByLabelText(/Close/i);
			await fireEvent.click(closeButton);

			expect(onclose).toHaveBeenCalledTimes(1);
		});
	});

	describe('backdrop', () => {
		it('renders overlay backdrop', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn()
			});
			const overlay = container.querySelector('.modal-overlay');
			expect(overlay).toBeInTheDocument();
		});

		it('calls onclose when clicking overlay', async () => {
			const onclose = vi.fn();
			const { container } = render(Modal, {
				open: true,
				onclose
			});

			const overlay = container.querySelector('.modal-overlay');
			await fireEvent.click(overlay!);

			expect(onclose).toHaveBeenCalledTimes(1);
		});

		it('does not call onclose when clicking modal content', async () => {
			const onclose = vi.fn();
			const { container } = render(Modal, {
				open: true,
				onclose
			});

			const modal = container.querySelector('.modal');
			await fireEvent.click(modal!);

			expect(onclose).not.toHaveBeenCalled();
		});
	});

	describe('keyboard interactions', () => {
		it('calls onclose when Escape key is pressed', async () => {
			const onclose = vi.fn();
			render(Modal, {
				open: true,
				onclose
			});

			await fireEvent.keyDown(document, { key: 'Escape' });

			expect(onclose).toHaveBeenCalledTimes(1);
		});

		it('handles Escape key on overlay element', async () => {
			const onclose = vi.fn();
			const { container } = render(Modal, {
				open: true,
				onclose
			});

			const overlay = container.querySelector('.modal-overlay');
			await fireEvent.keyDown(overlay!, { key: 'Escape' });

			expect(onclose).toHaveBeenCalledTimes(1);
		});
	});

	describe('body scroll lock', () => {
		it('prevents body scroll when modal opens', async () => {
			render(Modal, {
				open: true,
				onclose: vi.fn()
			});

			await waitFor(() => {
				expect(document.body.style.overflow).toBe('hidden');
			});
		});

		it('restores body scroll when modal closes', async () => {
			const { rerender } = render(Modal, {
				open: true,
				onclose: vi.fn()
			});

			// Wait for scroll lock to apply
			await waitFor(() => {
				expect(document.body.style.overflow).toBe('hidden');
			});

			// Close modal
			await rerender({ open: false, onclose: vi.fn() });

			await waitFor(() => {
				expect(document.body.style.overflow).toBe('');
			});
		});
	});

	describe('footer', () => {
		it('renders footer content', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				footer: 'Footer content'
			});

			expect(screen.getByText('Footer content')).toBeInTheDocument();
		});

		it('renders footer in correct element', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn(),
				footer: 'Footer content'
			});
			const footer = container.querySelector('.modal-footer');
			expect(footer).toBeInTheDocument();
			expect(footer).toHaveTextContent('Footer content');
		});
	});

	describe('accessibility', () => {
		it('has role="dialog"', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn()
			});

			expect(screen.getByRole('dialog')).toBeInTheDocument();
		});

		it('has aria-modal="true"', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn()
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-modal', 'true');
		});

		it('has aria-labelledby when title is provided', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				title: 'Test Modal'
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-labelledby', 'modal-title');
		});

		it('has aria-describedby when describedby is provided', () => {
			render(Modal, {
				open: true,
				onclose: vi.fn(),
				describedby: 'modal-description'
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-describedby', 'modal-description');
		});

		it('has tabindex="-1" for focus management', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn()
			});
			const modal = container.querySelector('.modal');
			expect(modal).toHaveAttribute('tabindex', '-1');
		});
	});

	describe('custom content class', () => {
		it('applies custom class to modal body', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn(),
				contentClass: 'custom-content-class'
			});
			const body = container.querySelector('.modal-body');
			expect(body).toHaveClass('custom-content-class');
		});
	});

	describe('stacked modal', () => {
		it('applies stacked class for higher z-index', () => {
			const { container } = render(Modal, {
				open: true,
				onclose: vi.fn(),
				stacked: true
			});
			const overlay = container.querySelector('.modal-overlay');
			expect(overlay).toHaveClass('top');
		});
	});
});
