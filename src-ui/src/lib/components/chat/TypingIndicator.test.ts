import { render, screen } from '@testing-library/svelte';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import TypingIndicator from './TypingIndicator.svelte';

describe('TypingIndicator', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the indicator container', () => {
			const { container } = render(TypingIndicator);
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('renders three dots by default', () => {
			const { container } = render(TypingIndicator);
			const dots = container.querySelectorAll('.dot, [class*="dot"]');
			expect(dots.length).toBe(3);
		});
	});

	describe('variants', () => {
		it('renders dots variant', () => {
			const { container } = render(TypingIndicator, {
				variant: 'dots'
			});
			const dots = container.querySelectorAll('.dot, [class*="dot"]');
			expect(dots.length).toBe(3);
		});

		it('renders text variant', () => {
			render(TypingIndicator, {
				variant: 'text',
				text: 'Typing'
			});
			expect(screen.getByText('Typing')).toBeInTheDocument();
		});

		it('renders combined variant (default)', () => {
			render(TypingIndicator, {
				variant: 'combined',
				text: 'Answering',
				showLabel: true
			});
			expect(screen.getByText('Answering...')).toBeInTheDocument();
			const { container } = render(TypingIndicator, {
				variant: 'combined'
			});
			const dots = container.querySelectorAll('.dot, [class*="dot"]');
			expect(dots.length).toBe(3);
		});
	});

	describe('text label', () => {
		it('displays custom text when provided', () => {
			render(TypingIndicator, {
				text: 'Thinking...',
				showLabel: true
			});
			expect(screen.getByText('Thinking...')).toBeInTheDocument();
		});

		it('hides label when showLabel is false', () => {
			render(TypingIndicator, {
				text: 'Thinking',
				showLabel: false,
				variant: 'combined'
			});
			expect(screen.queryByText('Thinking...')).not.toBeInTheDocument();
		});

		it('uses default text when not provided', () => {
			render(TypingIndicator, {
				showLabel: true,
				variant: 'combined'
			});
			expect(screen.getByText('Answering...')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="status"', () => {
			const { container } = render(TypingIndicator);
			const statusEl = container.querySelector('[role="status"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('has aria-live="polite"', () => {
			const { container } = render(TypingIndicator);
			const statusEl = container.querySelector('[aria-live="polite"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('has descriptive aria-label', () => {
			const { container } = render(TypingIndicator);
			const statusEl = container.querySelector('[aria-label="Assistant is typing"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('marks dots as aria-hidden', () => {
			const { container } = render(TypingIndicator);
			const dotsContainer = container.querySelector('[aria-hidden="true"]');
			expect(dotsContainer).toBeInTheDocument();
		});
	});

	describe('CSS classes', () => {
		it('applies variant-specific class', () => {
			const { container } = render(TypingIndicator, {
				variant: 'dots'
			});
			const indicator = container.querySelector('.dots, [class*="dots"]');
			expect(indicator).toBeInTheDocument();
		});

		it('applies combined variant class', () => {
			const { container } = render(TypingIndicator, {
				variant: 'combined'
			});
			const indicator = container.querySelector('.combined, [class*="combined"]');
			expect(indicator).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty text', () => {
			const { container } = render(TypingIndicator, {
				text: '',
				showLabel: true
			});
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('handles very long text', () => {
			const longText = 'A'.repeat(200);
			render(TypingIndicator, {
				text: longText,
				showLabel: true
			});
			expect(screen.getByText(longText)).toBeInTheDocument();
		});

		it('handles special characters in text', () => {
			const specialText = '<script>alert("test")</script>';
			render(TypingIndicator, {
				text: specialText,
				showLabel: true
			});
			// Text should be displayed as is (not executed as HTML)
			const indicator = screen.getByRole('status');
			expect(indicator).toBeInTheDocument();
		});
	});
});
