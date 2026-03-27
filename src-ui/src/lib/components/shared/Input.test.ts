import {fireEvent, render, screen} from '@testing-library/svelte';
import {describe, expect, it, vi} from 'vitest';
import Input from './Input.svelte';

describe('Input', () => {
	describe('rendering', () => {
		it('renders input element', () => {
			render(Input, {
				value: '',
				onchange: vi.fn()
			});

			const input = screen.getByRole('textbox');
			expect(input).toBeInTheDocument();
		});

		it('renders with initial value', () => {
			render(Input, {
				value: 'test value',
				onchange: vi.fn()
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveValue('test value');
		});

		it('renders label when provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				label: 'Username'
			});

			expect(screen.getByText('Username')).toBeInTheDocument();
		});

		it('does not render label when not provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn()
			});

			const label = screen.queryByLabelText(/.*/);
			expect(label).not.toBeInTheDocument();
		});
	});

	describe('input types', () => {
		it('renders text input by default', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				type: 'text'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('type', 'text');
		});

		it('renders password input', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				type: 'password',
				name: 'password'
			});

			const input = screen.getByRole('textbox') || screen.querySelector('input[type="password"]');
			expect(input).toHaveAttribute('type', 'password');
		});

		it('renders email input', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				type: 'email'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('type', 'email');
		});

		it('renders url input', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				type: 'url'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('type', 'url');
		});
	});

	describe('input handling', () => {
		it('calls onchange when value changes', async () => {
			const handleChange = vi.fn();
			render(Input, {
				value: '',
				onchange: handleChange
			});

			const input = screen.getByRole('textbox');
			await fireEvent.input(input, { target: { value: 'new value' } });

			expect(handleChange).toHaveBeenCalledWith('new value');
		});

		it('updates from external value changes', async () => {
			const { rerender } = render(Input, {
				value: 'initial',
				onchange: vi.fn()
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveValue('initial');

			await rerender({ value: 'updated', onchange: vi.fn() });
			expect(input).toHaveValue('updated');
		});
	});

	describe('placeholder', () => {
		it('renders placeholder when provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				placeholder: 'Enter text here'
			});

			const input = screen.getByPlaceholderText('Enter text here');
			expect(input).toBeInTheDocument();
		});
	});

	describe('helper text', () => {
		it('renders helper text when provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				helper: 'This is helper text'
			});

			expect(screen.getByText('This is helper text')).toBeInTheDocument();
		});

		it('associates helper with input via aria-describedby', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'test-input',
				helper: 'Helper text'
			});

			const input = screen.getByRole('textbox');
			const helper = screen.getByText('Helper text');

			expect(input).toHaveAttribute('aria-describedby', 'test-input-helper');
			expect(helper).toHaveAttribute('id', 'test-input-helper');
		});
	});

	describe('error state', () => {
		it('renders error message when provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				error: 'This field is required'
			});

			expect(screen.getByText('This field is required')).toBeInTheDocument();
		});

		it('applies error class to input', () => {
			const { container } = render(Input, {
				value: '',
				onchange: vi.fn(),
				error: 'Error message'
			});

			const input = container.querySelector('input');
			expect(input).toHaveClass('error');
		});

		it('has aria-invalid when error is present', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'test-input',
				error: 'Error message'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('aria-invalid', 'true');
		});

		it('associates error with input via aria-describedby', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'test-input',
				error: 'Error message'
			});

			const input = screen.getByRole('textbox');
			const error = screen.getByText('Error message');

			expect(input).toHaveAttribute('aria-describedby', 'test-input-error');
			expect(error).toHaveAttribute('id', 'test-input-error');
		});

		it('shows error instead of helper when both are provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'test-input',
				helper: 'Helper text',
				error: 'Error message'
			});

			expect(screen.getByText('Error message')).toBeInTheDocument();
			expect(screen.queryByText('Helper text')).not.toBeInTheDocument();
		});
	});

	describe('disabled state', () => {
		it('is enabled by default', () => {
			render(Input, {
				value: '',
				onchange: vi.fn()
			});

			const input = screen.getByRole('textbox');
			expect(input).not.toBeDisabled();
		});

		it('is disabled when disabled prop is true', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				disabled: true
			});

			const input = screen.getByRole('textbox');
			expect(input).toBeDisabled();
		});
	});

	describe('required state', () => {
		it('is not required by default', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				label: 'Test Label'
			});

			const input = screen.getByRole('textbox');
			expect(input).not.toBeRequired();
		});

		it('is required when required prop is true', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				required: true
			});

			const input = screen.getByRole('textbox');
			expect(input).toBeRequired();
		});

		it('shows asterisk next to label when required', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				label: 'Username',
				required: true
			});

			const asterisk = screen.getByText('*');
			expect(asterisk).toBeInTheDocument();
		});
	});

	describe('autofocus', () => {
		it('does not autofocus by default', () => {
			render(Input, {
				value: '',
				onchange: vi.fn()
			});

			const input = screen.getByRole('textbox');
			expect(input).not.toHaveFocus();
		});

		it('autofocuses when autofocus prop is true', async () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				autofocus: true
			});

			// Wait for the $effect to run
			await new Promise((resolve) => setTimeout(resolve, 0));

			const input = screen.getByRole('textbox');
			expect(input).toHaveFocus();
		});
	});

	describe('name attribute', () => {
		it('sets name attribute when provided', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'username'
			});

			const input = screen.getByRole('textbox');
			expect(input).toHaveAttribute('name', 'username');
		});
	});

	describe('custom class', () => {
		it('applies custom input class', () => {
			const { container } = render(Input, {
				value: '',
				onchange: vi.fn(),
				inputClass: 'my-custom-input'
			});

			const input = container.querySelector('input');
			expect(input).toHaveClass('my-custom-input');
		});
	});

	describe('accessibility', () => {
		it('associates label with input via for attribute', () => {
			render(Input, {
				value: '',
				onchange: vi.fn(),
				name: 'test-input',
				label: 'Test Label'
			});

			const label = screen.getByText('Test Label');
			const input = screen.getByRole('textbox');

			expect(label).toHaveAttribute('for', 'test-input');
			expect(input).toHaveAttribute('name', 'test-input');
		});
	});
});
