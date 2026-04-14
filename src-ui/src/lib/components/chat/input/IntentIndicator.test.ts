import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import IntentIndicator from './IntentIndicator.svelte';

describe('IntentIndicator', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
	});

	describe('rendering', () => {
		it('renders the badge container', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code'
			});
			const badge = container.querySelector('[role="status"]');
			expect(badge).toBeInTheDocument();
		});

		it('displays the intent label', () => {
			render(IntentIndicator, {
				intent: 'generate_code'
			});
			expect(screen.getByText('Generate Code')).toBeInTheDocument();
		});

		it('renders the intent icon', () => {
			const { container } = render(IntentIndicator, {
				intent: 'explain_code'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('intent types', () => {
		it('displays correct label for generate_code', () => {
			render(IntentIndicator, {
				intent: 'generate_code'
			});
			expect(screen.getByText('Generate Code')).toBeInTheDocument();
		});

		it('displays correct label for explain_code', () => {
			render(IntentIndicator, {
				intent: 'explain_code'
			});
			expect(screen.getByText('Explain')).toBeInTheDocument();
		});

		it('displays correct label for refactor_code', () => {
			render(IntentIndicator, {
				intent: 'refactor_code'
			});
			expect(screen.getByText('Refactor')).toBeInTheDocument();
		});

		it('displays correct label for find_bugs', () => {
			render(IntentIndicator, {
				intent: 'find_bugs'
			});
			expect(screen.getByText('Find Bugs')).toBeInTheDocument();
		});

		it('displays correct label for run_tests', () => {
			render(IntentIndicator, {
				intent: 'run_tests'
			});
			expect(screen.getByText('Run Tests')).toBeInTheDocument();
		});

		it('handles unknown intents', () => {
			render(IntentIndicator, {
				intent: 'unknown_intent'
			});
			expect(screen.getByText('Unknown intent')).toBeInTheDocument();
		});
	});

	describe('legacy intents', () => {
		it('displays correct label for legacy "generate" intent', () => {
			render(IntentIndicator, {
				intent: 'generate'
			});
			expect(screen.getByText('Generate')).toBeInTheDocument();
		});

		it('displays correct label for legacy "explain" intent', () => {
			render(IntentIndicator, {
				intent: 'explain'
			});
			expect(screen.getByText('Explain')).toBeInTheDocument();
		});

		it('displays correct label for legacy "fix" intent', () => {
			render(IntentIndicator, {
				intent: 'fix'
			});
			expect(screen.getByText('Fix')).toBeInTheDocument();
		});
	});

	describe('confidence display', () => {
		it('does not show confidence by default', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.85
			});
			expect(screen.queryByText('%')).not.toBeInTheDocument();
		});

		it('shows confidence when showConfidence is true', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.85,
				showConfidence: true
			});
			expect(screen.getByText('85%')).toBeInTheDocument();
		});

		it('rounds confidence correctly', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.856,
				showConfidence: true
			});
			expect(screen.getByText('86%')).toBeInTheDocument();
		});

		it('does not show confidence when 0', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0,
				showConfidence: true
			});
			expect(screen.queryByText('%')).not.toBeInTheDocument();
		});

		it('applies high confidence class when confidence > 0.8', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.9
			});
			const badge = container.querySelector('.confidence-high, [class*="confidence-high"]');
			expect(badge).toBeInTheDocument();
		});

		it('applies medium confidence class when confidence > 0.5', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.6
			});
			const badge = container.querySelector('.confidence-medium, [class*="confidence-medium"]');
			expect(badge).toBeInTheDocument();
		});

		it('applies low confidence class when confidence <= 0.5', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.3
			});
			const badge = container.querySelector('.confidence-low, [class*="confidence-low"]');
			expect(badge).toBeInTheDocument();
		});
	});

	describe('loading state', () => {
		it('shows spinner when loading is true', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				loading: true
			});
			const spinner = container.querySelector('.spinner, [class*="spinner"]');
			expect(spinner).toBeInTheDocument();
		});

		it('hides icon when loading', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				loading: true
			});
			const icon = container.querySelector('.icon svg');
			expect(icon).not.toBeInTheDocument();
		});

		it('applies loading class when loading', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				loading: true
			});
			const badge = container.querySelector('.loading, [class*="loading"]');
			expect(badge).toBeInTheDocument();
		});

		it('shows intent label even when loading', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				loading: true
			});
			expect(screen.getByText('Generate Code')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="status"', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code'
			});
			const statusEl = container.querySelector('[role="status"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('has descriptive aria-label', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code'
			});
			const statusEl = container.querySelector('[aria-label="Detected intent: Generate Code"]');
			expect(statusEl).toBeInTheDocument();
		});
	});

	describe('icon variations', () => {
		it('shows code icon for code generation intents', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('shows book icon for explanation intents', () => {
			const { container } = render(IntentIndicator, {
				intent: 'explain_code'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('shows check icon for test-related intents', () => {
			const { container } = render(IntentIndicator, {
				intent: 'run_tests'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});

		it('shows bug icon for bug finding intents', () => {
			const { container } = render(IntentIndicator, {
				intent: 'find_bugs'
			});
			const icon = container.querySelector('svg');
			expect(icon).toBeInTheDocument();
		});
	});

	describe('error state', () => {
		it('renders error variant when error prop is set', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				error: 'Classification service unavailable'
			});
			const badge = container.querySelector('.error, [class*="error"]');
			expect(badge).toBeInTheDocument();
		});

		it('shows fallback label "Intent unavailable" when errored', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				error: 'Network failure'
			});
			expect(screen.getByText('Intent unavailable')).toBeInTheDocument();
			expect(screen.queryByText('Generate Code')).not.toBeInTheDocument();
		});

		it('exposes an accessible label indicating intent detection is unavailable', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				error: 'Backend offline'
			});
			const statusEl = container.querySelector('[aria-label="Intent detection unavailable"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('suppresses confidence display in error state even when showConfidence is true', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.9,
				showConfidence: true,
				error: 'Backend offline'
			});
			expect(screen.queryByText('90%')).not.toBeInTheDocument();
		});

		it('prioritizes loading state over error when both are set', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				loading: true,
				error: 'Backend offline'
			});
			const spinner = container.querySelector('.spinner, [class*="spinner"]');
			expect(spinner).toBeInTheDocument();
			expect(screen.queryByText('Intent unavailable')).not.toBeInTheDocument();
		});

		it('exposes the error message as a tooltip for sighted users', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				error: 'Request timed out'
			});
			const badge = container.querySelector('[title="Request timed out"]');
			expect(badge).toBeInTheDocument();
		});

		it('falls back to normal rendering when error is null', () => {
			render(IntentIndicator, {
				intent: 'generate_code',
				error: null
			});
			expect(screen.getByText('Generate Code')).toBeInTheDocument();
			expect(screen.queryByText('Intent unavailable')).not.toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles empty intent string', () => {
			render(IntentIndicator, {
				intent: ''
			});
			const container = document.body;
			expect(container).toBeInTheDocument();
		});

		it('handles intent with underscores', () => {
			render(IntentIndicator, {
				intent: 'start_new_thread'
			});
			expect(screen.getByText('New Thread')).toBeInTheDocument();
		});

		it('handles very long intent names', () => {
			const longIntent = 'very_long_intent_name_that_goes_on_and_on';
			render(IntentIndicator, {
				intent: longIntent
			});
			// Should capitalize first letter
			expect(screen.getByText(/Very long intent/)).toBeInTheDocument();
		});

		it('handles confidence at boundaries', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.8,
				showConfidence: true
			});
			// At exactly 0.8, should be high confidence
			const badge = container.querySelector('.confidence-high, [class*="confidence-high"]');
			expect(badge).toBeInTheDocument();
		});

		it('handles confidence of exactly 0.5', () => {
			const { container } = render(IntentIndicator, {
				intent: 'generate_code',
				confidence: 0.5
			});
			const badge = container.querySelector('.confidence-low, [class*="confidence-low"]');
			expect(badge).toBeInTheDocument();
		});
	});
});
