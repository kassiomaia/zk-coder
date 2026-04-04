import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import Button from './Button.svelte';

describe('Button', () => {
	describe('rendering', () => {
		it('renders button element', () => {
			render(Button, { children: 'Click me' });
			const button = screen.getByRole('button');
			expect(button).toBeInTheDocument();
		});

		it('renders children text', () => {
			render(Button, { children: 'Click me' });
			expect(screen.getByText('Click me')).toBeInTheDocument();
		});

		it('renders with default variant', () => {
			const { container } = render(Button, { children: 'Click me' });
			const button = container.querySelector('button');
			expect(button).toHaveClass('primary');
		});
	});

	describe('variants', () => {
		it('applies primary variant', () => {
			const { container } = render(Button, {
				variant: 'primary',
				children: 'Primary'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('primary');
		});

		it('applies accent variant', () => {
			const { container } = render(Button, {
				variant: 'accent',
				children: 'Accent'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('accent');
		});

		it('applies secondary variant', () => {
			const { container } = render(Button, {
				variant: 'secondary',
				children: 'Secondary'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('secondary');
		});

		it('applies ghost variant', () => {
			const { container } = render(Button, {
				variant: 'ghost',
				children: 'Ghost'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('ghost');
		});

		it('applies danger variant', () => {
			const { container } = render(Button, {
				variant: 'danger',
				children: 'Danger'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('danger');
		});
	});

	describe('sizes', () => {
		it('applies default size', () => {
			const { container } = render(Button, { children: 'Default' });
			const button = container.querySelector('button');
			expect(button).toHaveClass('default');
		});

		it('applies small size', () => {
			const { container } = render(Button, {
				size: 'sm',
				children: 'Small'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('sm');
		});

		it('applies large size', () => {
			const { container } = render(Button, {
				size: 'lg',
				children: 'Large'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('lg');
		});
	});

	describe('disabled state', () => {
		it('is enabled by default', () => {
			render(Button, { children: 'Click me' });
			const button = screen.getByRole('button');
			expect(button).not.toBeDisabled();
		});

		it('is disabled when disabled prop is true', () => {
			render(Button, { disabled: true, children: 'Disabled' });
			const button = screen.getByRole('button');
			expect(button).toBeDisabled();
		});

		it('has aria-disabled attribute when disabled', () => {
			render(Button, { disabled: true, children: 'Disabled' });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-disabled', 'true');
		});

		it('does not call onclick when disabled', async () => {
			const handleClick = vi.fn();
			render(Button, {
				disabled: true,
				onclick: handleClick,
				children: 'Disabled'
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).not.toHaveBeenCalled();
		});
	});

	describe('loading state', () => {
		it('shows loading spinner when loading is true', () => {
			const { container } = render(Button, {
				loading: true,
				children: 'Loading'
			});
			const spinner = container.querySelector('.spinner');
			expect(spinner).toBeInTheDocument();
		});

		it('has aria-busy attribute when loading', () => {
			render(Button, { loading: true, children: 'Loading' });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-busy', 'true');
		});

		it('does not call onclick when loading', async () => {
			const handleClick = vi.fn();
			render(Button, {
				loading: true,
				onclick: handleClick,
				children: 'Loading'
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).not.toHaveBeenCalled();
		});

		it('is disabled when loading', () => {
			render(Button, { loading: true, children: 'Loading' });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-disabled', 'true');
		});
	});

	describe('icon only mode', () => {
		it('renders in icon only mode', () => {
			const { container } = render(Button, {
				iconOnly: true,
				children: 'Icon'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('iconOnly');
		});
	});

	describe('click interactions', () => {
		it('calls onclick handler when clicked', async () => {
			const handleClick = vi.fn();
			render(Button, {
				onclick: handleClick,
				children: 'Click me'
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).toHaveBeenCalledTimes(1);
		});

		it('handles mouse press state', async () => {
			const { container } = render(Button, { children: 'Press me' });
			const button = container.querySelector('button');

			await fireEvent.mouseDown(button!);
			await fireEvent.mouseUp(button!);

			// Button should still be present
			expect(button).toBeInTheDocument();
		});
	});

	describe('full width', () => {
		it('applies full width class when fullWidth is true', () => {
			const { container } = render(Button, {
				fullWidth: true,
				children: 'Full Width'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('fullWidth');
		});
	});

	describe('accessibility', () => {
		it('applies aria-label when provided', () => {
			render(Button, {
				'aria-label': 'Close dialog',
				children: '×'
			});
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-label', 'Close dialog');
		});

		it('applies aria-describedby when provided', () => {
			render(Button, {
				'aria-describedby': 'button-description',
				children: 'Click me'
			});
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-describedby', 'button-description');
		});

		it('has correct type attribute', () => {
			render(Button, { type: 'submit', children: 'Submit' });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('type', 'submit');
		});
	});

	describe('custom class', () => {
		it('applies custom className', () => {
			const { container } = render(Button, {
				class: 'my-custom-class',
				children: 'Custom'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('my-custom-class');
		});
	});

	describe('animation', () => {
		it('applies subtle animation by default', () => {
			const { container } = render(Button, { children: 'Animated' });
			const button = container.querySelector('button');
			expect(button).toHaveClass('subtle');
		});

		it('applies none animation when specified', () => {
			const { container } = render(Button, {
				animation: 'none',
				children: 'No Animation'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('none');
		});

		it('applies expressive animation when specified', () => {
			const { container } = render(Button, {
				animation: 'expressive',
				children: 'Expressive'
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('expressive');
		});
	});
});
