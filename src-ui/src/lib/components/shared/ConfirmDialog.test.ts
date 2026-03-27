import {fireEvent, render, screen} from '@testing-library/svelte';
import {beforeEach, describe, expect, it, vi} from 'vitest';
import ConfirmDialog from './ConfirmDialog.svelte';

describe('ConfirmDialog', () => {
	let mockOnConfirm: ReturnType<typeof vi.fn>;
	let mockOnCancel: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockOnConfirm = vi.fn();
		mockOnCancel = vi.fn();
		vi.clearAllMocks();
	});

	describe('rendering', () => {
		it('renders modal when open is true', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm Action',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByRole('dialog')).toBeInTheDocument();
			expect(screen.getByText('Confirm Action')).toBeInTheDocument();
		});

		it('does not render modal when open is false', () => {
			render(ConfirmDialog, {
				open: false,
				title: 'Confirm Action',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
		});

		it('renders confirmation message', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Delete Item',
				message: 'This action cannot be undone.',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByText('This action cannot be undone.')).toBeInTheDocument();
		});

		it('renders default button labels', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByRole('button', { name: 'Cancel' })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: 'Confirm' })).toBeInTheDocument();
		});

		it('renders custom button labels', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Delete',
				message: 'Delete this item?',
				confirmLabel: 'Delete',
				cancelLabel: 'Keep',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByRole('button', { name: 'Keep' })).toBeInTheDocument();
			expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
		});

		it('has aria-describedby pointing to message element', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-describedby', 'confirm-dialog-message');
		});
	});

	describe('button actions', () => {
		it('calls onconfirm when confirm button is clicked', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			await fireEvent.click(confirmButton);

			expect(mockOnConfirm).toHaveBeenCalledTimes(1);
		});

		it('calls oncancel when cancel button is clicked', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockOnCancel).toHaveBeenCalledTimes(1);
		});

		it('does not call onconfirm when clicking cancel button', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockOnConfirm).not.toHaveBeenCalled();
		});
	});

	describe('loading state', () => {
		it('disables both buttons when loading is true', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel,
				loading: true
			});

			const buttons = screen.getAllByRole('button');
			// Button uses aria-disabled instead of disabled attribute
			expect(buttons[0]).toHaveAttribute('aria-disabled', 'true');
			expect(buttons[1]).toHaveAttribute('aria-disabled', 'true');
		});

		it('prevents onconfirm call when loading', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel,
				loading: true
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			await fireEvent.click(confirmButton);

			expect(mockOnConfirm).not.toHaveBeenCalled();
		});

		it('prevents oncancel call when loading', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel,
				loading: true
			});

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockOnCancel).not.toHaveBeenCalled();
		});

		it('shows loading indicator on confirm button', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel,
				loading: true
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			expect(confirmButton).toHaveAttribute('aria-busy', 'true');
		});
	});

	describe('variant prop', () => {
		it('renders danger variant dialog', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Delete',
				message: 'Delete this item?',
				variant: 'danger',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			expect(confirmButton).toBeInTheDocument();
		});

		it('renders primary variant dialog', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				variant: 'primary',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			expect(confirmButton).toBeInTheDocument();
		});

		it('renders warning variant dialog', () => {
			// Note: In the component, warning is mapped to primary
			render(ConfirmDialog, {
				open: true,
				title: 'Warning',
				message: 'This is a warning?',
				variant: 'warning',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			expect(confirmButton).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="dialog"', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByRole('dialog')).toBeInTheDocument();
		});

		it('has aria-modal="true"', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-modal', 'true');
		});

		it('has aria-describedby linking to message', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure you want to proceed?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const modal = screen.getByRole('dialog');
			expect(modal).toHaveAttribute('aria-describedby', 'confirm-dialog-message');
		});

		it('message has id matching aria-describedby', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const message = screen.getByText('Are you sure?');
			expect(message).toHaveAttribute('id', 'confirm-dialog-message');
		});

		it('cancel button is rendered', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			expect(cancelButton).toBeInTheDocument();
		});
	});

	describe('footer content', () => {
		it('renders buttons in footer', () => {
			const { container } = render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const footer = container.querySelector('footer');
			expect(footer).toBeInTheDocument();
			expect(footer?.children.length).toBeGreaterThan(0);
		});

		it('has both buttons in footer in correct order', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const buttons = screen.getAllByRole('button');
			expect(buttons).toHaveLength(2);
			expect(buttons[0]).toHaveTextContent('Cancel'); // Cancel comes first
			expect(buttons[1]).toHaveTextContent('Confirm'); // Confirm comes second
		});
	});

	describe('use cases', () => {
		it('renders delete confirmation dialog', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Delete Provider',
				message: 'Are you sure you want to delete this provider? This action cannot be undone.',
				confirmLabel: 'Delete',
				cancelLabel: 'Cancel',
				variant: 'danger',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByText('Delete Provider')).toBeInTheDocument();
			expect(screen.getByText(/are you sure/i)).toBeInTheDocument();
			expect(screen.getByRole('button', { name: 'Delete' })).toBeInTheDocument();
		});

		it('renders archive confirmation dialog', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Archive Thread',
				message: 'Archive this thread? It will be moved to the archive.',
				confirmLabel: 'Archive',
				cancelLabel: 'Cancel',
				variant: 'primary',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByText('Archive Thread')).toBeInTheDocument();
			expect(screen.getByRole('button', { name: 'Archive' })).toBeInTheDocument();
		});

		it('renders warning confirmation dialog', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Unsaved Changes',
				message: 'You have unsaved changes. Do you want to continue?',
				confirmLabel: 'Continue',
				cancelLabel: 'Stay',
				variant: 'warning',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			expect(screen.getByText('Unsaved Changes')).toBeInTheDocument();
			expect(screen.getByRole('button', { name: 'Continue' })).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty message gracefully', () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: '',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const messageEl = document.getElementById('confirm-dialog-message');
			expect(messageEl).toBeInTheDocument();
			expect(messageEl?.textContent).toBe('');
		});

		it('handles long message text', () => {
			const longMessage = 'This is a very long confirmation message. '.repeat(10);

			const { container } = render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: longMessage,
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const messageEl = container.querySelector('#confirm-dialog-message');
			expect(messageEl).toBeInTheDocument();
			expect(messageEl?.textContent).toContain('This is a very long confirmation message');
		});

		it('handles special characters in message', () => {
			const specialMessage = 'Are you sure you want to delete <script>alert("xss")</script>?';

			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: specialMessage,
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			// Text should be properly escaped
			expect(screen.getByText(/alert/i)).toBeInTheDocument();
		});
	});

	describe('multiple rapid clicks', () => {
		it('only calls onconfirm once per click', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const confirmButton = screen.getByRole('button', { name: 'Confirm' });
			await fireEvent.click(confirmButton);
			await fireEvent.click(confirmButton);
			await fireEvent.click(confirmButton);

			expect(mockOnConfirm).toHaveBeenCalledTimes(3);
		});

		it('only calls oncancel once per click', async () => {
			render(ConfirmDialog, {
				open: true,
				title: 'Confirm',
				message: 'Are you sure?',
				onconfirm: mockOnConfirm,
				oncancel: mockOnCancel
			});

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);
			await fireEvent.click(cancelButton);

			expect(mockOnCancel).toHaveBeenCalledTimes(2);
		});
	});
});
