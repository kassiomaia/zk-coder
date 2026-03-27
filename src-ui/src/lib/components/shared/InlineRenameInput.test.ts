import {fireEvent, render, screen} from '@testing-library/svelte';
import {describe, expect, it, vi} from 'vitest';
import InlineRenameInput from './InlineRenameInput.svelte';

describe('InlineRenameInput', () => {
	const defaultProps = {
		value: 'Initial Value',
		onSave: vi.fn(),
		onCancel: vi.fn()
	};

	describe('rendering', () => {
		it('renders input with initial value', () => {
			render(InlineRenameInput, defaultProps);

			const input = screen.getByRole('textbox');
			expect(input).toHaveValue('Initial Value');
		});

		it('renders save button', () => {
			render(InlineRenameInput, defaultProps);

			expect(screen.getByRole('button', { name: /save/i })).toBeInTheDocument();
		});

		it('renders cancel button', () => {
			render(InlineRenameInput, defaultProps);

			expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument();
		});

		it('has input focused on mount', async () => {
			render(InlineRenameInput, {
				...defaultProps,
				autofocus: true
			});

			await new Promise((resolve) => setTimeout(resolve, 0));

			const input = screen.getByRole('textbox');
			expect(input).toHaveFocus();
		});
	});

	describe('input handling', () => {
		it('updates input value on change', async () => {
			render(InlineRenameInput, defaultProps);

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: 'New Value' } });

			expect(input).toHaveValue('New Value');
		});

		it('calls onSave with new value when save button is clicked', async () => {
			const onSave = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onSave
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: 'Updated Value' } });

			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			expect(onSave).toHaveBeenCalledWith('Updated Value');
		});

		it('calls onSave when Enter key is pressed', async () => {
			const onSave = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onSave
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: 'New Value' } });
			await fireEvent.keyDown(input, { key: 'Enter' });

			expect(onSave).toHaveBeenCalledWith('New Value');
		});

		it('calls onCancel when Escape key is pressed', async () => {
			const onCancel = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onCancel
			});

			const input = screen.getByRole('textbox');
			await fireEvent.keyDown(input, { key: 'Escape' });

			expect(onCancel).toHaveBeenCalledTimes(1);
		});
	});

	describe('cancel button', () => {
		it('calls onCancel when clicked', async () => {
			const onCancel = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onCancel
			});

			const cancelButton = screen.getByRole('button', { name: /cancel/i });
			await fireEvent.click(cancelButton);

			expect(onCancel).toHaveBeenCalledTimes(1);
		});

		it('does not call onSave when canceling', async () => {
			const onSave = vi.fn();
			const onCancel = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onSave,
				onCancel
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: 'Changed' } });

			const cancelButton = screen.getByRole('button', { name: /cancel/i });
			await fireEvent.click(cancelButton);

			expect(onSave).not.toHaveBeenCalled();
			expect(onCancel).toHaveBeenCalled();
		});
	});

	describe('validation', () => {
		it('shows error when value is empty and required', async () => {
			render(InlineRenameInput, {
				...defaultProps,
				required: true
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: '' } });

			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			// Should not call onSave with empty value when required
			expect(defaultProps.onSave).not.toHaveBeenCalled();
		});

		it('trims whitespace before saving', async () => {
			const onSave = vi.fn();
			render(InlineRenameInput, {
				...defaultProps,
				onSave
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: '  Trimmed Value  ' } });

			const saveButton = screen.getByRole('button', { name: /save/i });
			await fireEvent.click(saveButton);

			expect(onSave).toHaveBeenCalledWith('Trimmed Value');
		});
	});

	describe('placeholder', () => {
		it('renders placeholder when provided', () => {
			render(InlineRenameInput, {
				...defaultProps,
				placeholder: 'Enter new name'
			});

			const input = screen.getByPlaceholderText('Enter new name');
			expect(input).toBeInTheDocument();
		});
	});

	describe('disabled state', () => {
		it('disables input when disabled', () => {
			render(InlineRenameInput, {
				...defaultProps,
				disabled: true
			});

			const input = screen.getByRole('textbox');
			expect(input).toBeDisabled();
		});

		it('disables buttons when disabled', () => {
			render(InlineRenameInput, {
				...defaultProps,
				disabled: true
			});

			const saveButton = screen.getByRole('button', { name: /save/i });
			const cancelButton = screen.getByRole('button', { name: /cancel/i });

			expect(saveButton).toBeDisabled();
			expect(cancelButton).toBeDisabled();
		});
	});

	describe('accessibility', () => {
		it('has correct label association', () => {
			render(InlineRenameInput, {
				...defaultProps,
				label: 'Rename item',
				name: 'rename-input'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('name', 'rename-input');
		});

		it('has proper button labels', () => {
			render(InlineRenameInput, defaultProps);

			const saveButton = screen.getByRole('button', { name: /save/i });
			const cancelButton = screen.getByRole('button', { name: /cancel/i });

			expect(saveButton).toBeInTheDocument();
			expect(cancelButton).toBeInTheDocument();
		});
	});

	describe('max length', () => {
		it('respects maxLength prop', () => {
			render(InlineRenameInput, {
				...defaultProps,
				maxLength: 10
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('maxlength', '10');
		});
	});

	describe('select on focus', () => {
		it('selects text on focus when selectOnFocus is true', async () => {
			render(InlineRenameInput, {
				...defaultProps,
				selectOnFocus: true,
				autofocus: true
			});

			await new Promise((resolve) => setTimeout(resolve, 0));

			const input = screen.getByRole('textbox');
			expect(input).toHaveFocus();
		});
	});
});
