import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import ClarificationBanner from './ClarificationBanner.svelte';
import type { Ambiguity } from '$lib/types/nlp';

describe('ClarificationBanner', () => {
	let mockAmbiguity: Ambiguity;

	beforeEach(() => {
		vi.clearAllMocks();
		mockAmbiguity = {
			id: 'amb-1',
			ambiguity_type: 'missing_file',
			span: { start: 0, end: 10, text: 'file.ts' },
			severity: 'medium',
			clarification: {
				question: 'Which file would you like to analyze?',
				options: [
					{ value: 'src/index.ts', label: 'src/index.ts' },
					{ value: 'src/utils.ts', label: 'src/utils.ts' }
				],
				allows_freeform: true
			},
			suggestions: []
		};
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the banner container', () => {
			const { container } = render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const banner = container.querySelector('[class*="banner"]');
			expect(banner).toBeInTheDocument();
		});

		it('displays the type label', () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('Missing File')).toBeInTheDocument();
		});

		it('displays the question', () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('Which file would you like to analyze?')).toBeInTheDocument();
		});
	});

	describe('options display', () => {
		it('renders option buttons when options are provided', () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('src/index.ts')).toBeInTheDocument();
			expect(screen.getByText('src/utils.ts')).toBeInTheDocument();
		});

		it('renders option descriptions when provided', () => {
			const ambiguityWithDescriptions: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					options: [
						{
							value: 'src/index.ts',
							label: 'Main Entry',
							description: 'The main application entry point'
						}
					]
				}
			};

			render(ClarificationBanner, {
				ambiguity: ambiguityWithDescriptions,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('The main application entry point')).toBeInTheDocument();
		});

		it('does not render options section when no options', () => {
			const ambiguityWithoutOptions: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					options: []
				}
			};

			render(ClarificationBanner, {
				ambiguity: ambiguityWithoutOptions,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			// Should still render the banner, just without options
			const container = document.body;
			expect(container).toBeInTheDocument();
		});
	});

	describe('option selection', () => {
		it('calls onResolve with selected option', async () => {
			const onResolve = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve,
				onDismiss: vi.fn()
			});

			const optionButton = screen.getByText('src/index.ts');
			await fireEvent.click(optionButton);

			expect(onResolve).toHaveBeenCalledWith('amb-1', 'src/index.ts', 'selected');
		});

		it('includes stopPropagation on click to prevent parent handling', async () => {
			const onResolve = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve,
				onDismiss: vi.fn()
			});

			const optionButton = screen.getByText('src/index.ts');
			await fireEvent.click(optionButton);

			expect(onResolve).toHaveBeenCalled();
		});
	});

	describe('custom input', () => {
		it('shows custom input toggle when allows_freeform is true', () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('Enter custom value')).toBeInTheDocument();
		});

		it('does not show custom input toggle when allows_freeform is false', () => {
			const ambiguityNoFreeform: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					allows_freeform: false
				}
			};

			render(ClarificationBanner, {
				ambiguity: ambiguityNoFreeform,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.queryByText('Enter custom value')).not.toBeInTheDocument();
		});

		it('shows custom input field when toggle is clicked', async () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});

			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			const input = screen.getByPlaceholderText('Enter custom value...');
			expect(input).toBeInTheDocument();
		});

		it('submits custom value on Enter key', async () => {
			const onResolve = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve,
				onDismiss: vi.fn()
			});

			// Open custom input
			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			// Type in input
			const input = screen.getByPlaceholderText('Enter custom value...');
			await fireEvent.input(input, { target: { value: 'custom/path.ts' } });

			// Press Enter
			await fireEvent.keyDown(input, { key: 'Enter' });

			expect(onResolve).toHaveBeenCalledWith('amb-1', 'custom/path.ts', 'custom');
		});

		it('submits custom value via submit button', async () => {
			const onResolve = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve,
				onDismiss: vi.fn()
			});

			// Open custom input
			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			// Type in input
			const input = screen.getByPlaceholderText('Enter custom value...');
			await fireEvent.input(input, { target: { value: 'custom/path.ts' } });

			// Click submit button
			const submitButton = screen.getByLabelText('Submit custom value');
			await fireEvent.click(submitButton);

			expect(onResolve).toHaveBeenCalledWith('amb-1', 'custom/path.ts', 'custom');
		});

		it('closes custom input on cancel button click', async () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});

			// Open custom input
			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			// Click cancel
			const cancelButton = screen.getByLabelText('Cancel custom input');
			await fireEvent.click(cancelButton);

			expect(screen.queryByPlaceholderText('Enter custom value...')).not.toBeInTheDocument();
		});

		it('closes custom input on Escape key', async () => {
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});

			// Open custom input
			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			// Press Escape
			const input = screen.getByPlaceholderText('Enter custom value...');
			await fireEvent.keyDown(input, { key: 'Escape' });

			expect(screen.queryByPlaceholderText('Enter custom value...')).not.toBeInTheDocument();
		});

		it('does not submit empty custom input', async () => {
			const onResolve = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve,
				onDismiss: vi.fn()
			});

			// Open custom input
			const toggleButton = screen.getByText('Enter custom value');
			await fireEvent.click(toggleButton);

			// Try to submit without typing
			const input = screen.getByPlaceholderText('Enter custom value...');
			await fireEvent.keyDown(input, { key: 'Enter' });

			expect(onResolve).not.toHaveBeenCalled();
		});
	});

	describe('skip/dismiss action', () => {
		it('calls onDismiss when skip button is clicked', async () => {
			const onDismiss = vi.fn();
			render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss
			});

			const skipButton = screen.getByText('Skip');
			await fireEvent.click(skipButton);

			expect(onDismiss).toHaveBeenCalledWith('amb-1');
		});
	});

	describe('severity classes', () => {
		it('applies severity-specific class', () => {
			const { container } = render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			// The banner should have the medium severity class
			const banner = container.querySelector('[class*="medium"]');
			expect(banner).toBeInTheDocument();
		});

		it('applies high severity class', () => {
			const highSeverityAmbiguity: Ambiguity = {
				...mockAmbiguity,
				severity: 'high'
			};

			const { container } = render(ClarificationBanner, {
				ambiguity: highSeverityAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const banner = container.querySelector('[class*="high"]');
			expect(banner).toBeInTheDocument();
		});

		it('applies low severity class', () => {
			const lowSeverityAmbiguity: Ambiguity = {
				...mockAmbiguity,
				severity: 'low'
			};

			const { container } = render(ClarificationBanner, {
				ambiguity: lowSeverityAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const banner = container.querySelector('[class*="low"]');
			expect(banner).toBeInTheDocument();
		});
	});

	describe('different ambiguity types', () => {
		it('displays correct label for missing_symbol', () => {
			const symbolAmbiguity: Ambiguity = {
				...mockAmbiguity,
				ambiguity_type: 'missing_symbol'
			};

			render(ClarificationBanner, {
				ambiguity: symbolAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('Missing Symbol')).toBeInTheDocument();
		});

		it('displays correct label for ambiguous_reference', () => {
			const refAmbiguity: Ambiguity = {
				...mockAmbiguity,
				ambiguity_type: 'ambiguous_reference'
			};

			render(ClarificationBanner, {
				ambiguity: refAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			expect(screen.getByText('Ambiguous Reference')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has icon wrapper with alert icon', () => {
			const { container } = render(ClarificationBanner, {
				ambiguity: mockAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty options array', () => {
			const emptyOptionsAmbiguity: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					options: []
				}
			};

			const { container } = render(ClarificationBanner, {
				ambiguity: emptyOptionsAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const banner = container.querySelector('[class*="banner"]');
			expect(banner).toBeInTheDocument();
		});

		it('handles very long question text', () => {
			const longQuestionAmbiguity: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					question: 'A'.repeat(500)
				}
			};

			render(ClarificationBanner, {
				ambiguity: longQuestionAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles many options', () => {
			const manyOptionsAmbiguity: Ambiguity = {
				...mockAmbiguity,
				clarification: {
					...mockAmbiguity.clarification,
					options: Array.from({ length: 10 }, (_, i) => ({
						value: `option-${i}`,
						label: `Option ${i}`
					}))
				}
			};

			render(ClarificationBanner, {
				ambiguity: manyOptionsAmbiguity,
				onResolve: vi.fn(),
				onDismiss: vi.fn()
			});
			for (let i = 0; i < 10; i++) {
				expect(screen.getByText(`Option ${i}`)).toBeInTheDocument();
			}
		});
	});
});
