import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import AnsweringIndicator from './AnsweringIndicator.svelte';
import type { AgentStatus } from '$lib/utils/agentStatus';

describe('AnsweringIndicator', () => {
	const mockStatus: AgentStatus = {
		text: 'Analyzing code',
		active: true,
		icon: 'sparkle',
		phrases: []
	};

	beforeEach(() => {
		vi.useFakeTimers();
		vi.clearAllMocks();
	});

	afterEach(() => {
		vi.restoreAllMocks();
		vi.useRealTimers();
	});

	describe('rendering', () => {
		it('renders the container', () => {
			const { container } = render(AnsweringIndicator, { status: mockStatus });
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('renders status text', () => {
			render(AnsweringIndicator, { status: mockStatus });
			expect(screen.getByText(/Analyzing code/i)).toBeInTheDocument();
		});

		it('renders with active status', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, active: true }
			});
			const panel = container.querySelector('.panel, [class*="panel"]');
			expect(panel).toBeInTheDocument();
		});

		it('renders with inactive status (complete)', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, active: false }
			});
			const panel = container.querySelector('.complete, [class*="complete"]');
			expect(panel).toBeInTheDocument();
		});
	});

	describe('icon display', () => {
		it('renders sparkle icon by default', () => {
			const { container } = render(AnsweringIndicator, {
				status: mockStatus
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('renders scan icon when specified', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'scan' }
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('renders terminal icon when specified', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'terminal' }
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('renders file icon when specified', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'file' }
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});
	});

	describe('phrase rotation', () => {
		it('displays first phrase when multiple phrases provided', () => {
			const statusWithPhrases: AgentStatus = {
				...mockStatus,
				phrases: ['Reading files', 'Analyzing code', 'Generating response']
			};
			render(AnsweringIndicator, { status: statusWithPhrases });
			expect(screen.getByText(/Reading files/i)).toBeInTheDocument();
		});

		it('uses text as fallback when no phrases', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, text: 'Processing...', phrases: [] }
			});
			expect(screen.getByText(/Processing/i)).toBeInTheDocument();
		});
	});

	describe('stall warnings', () => {
		it('displays "Still working..." warning when stall-warning is still-working', () => {
			const status: AgentStatus = {
				...mockStatus,
				stallWarning: 'still-working',
				elapsedSeconds: 30
			};
			render(AnsweringIndicator, { status });
			expect(screen.getByText(/Still working/i)).toBeInTheDocument();
		});

		it('displays elapsed time when stall-warning is taking-longer', () => {
			const status: AgentStatus = {
				...mockStatus,
				stallWarning: 'taking-longer',
				elapsedSeconds: 120
			};
			render(AnsweringIndicator, { status });
			expect(screen.getByText(/Taking longer than expected/i)).toBeInTheDocument();
			expect(screen.getByText(/2:00/i)).toBeInTheDocument();
		});

		it('displays LLM waiting message when stall-warning is very-long', () => {
			const status: AgentStatus = {
				...mockStatus,
				stallWarning: 'very-long',
				elapsedSeconds: 300
			};
			render(AnsweringIndicator, { status });
			expect(screen.getByText(/Still waiting for LLM/i)).toBeInTheDocument();
		});
	});

	describe('animation states', () => {
		it('applies stable class when animation completes', async () => {
			const { container } = render(AnsweringIndicator, {
				status: mockStatus
			});

			// Wait for scramble animation to complete
			vi.advanceTimersByTime(1000);
			await tick();

			const panel = container.querySelector('.stable, [class*="stable"]');
			expect(panel).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="status"', () => {
			const { container } = render(AnsweringIndicator, {
				status: mockStatus
			});
			const statusEl = container.querySelector('[role="status"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('has aria-live="polite"', () => {
			const { container } = render(AnsweringIndicator, {
				status: mockStatus
			});
			const statusEl = container.querySelector('[aria-live="polite"]');
			expect(statusEl).toBeInTheDocument();
		});

		it('has descriptive aria-label', () => {
			const { container } = render(AnsweringIndicator, {
				status: mockStatus
			});
			const statusEl = container.querySelector('[aria-label="Agent status"]');
			expect(statusEl).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles null status gracefully', () => {
			const { container } = render(AnsweringIndicator, {
				status: null
			});
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('handles undefined status gracefully', () => {
			const { container } = render(AnsweringIndicator, {
				status: undefined
			});
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('handles empty text', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, text: '', phrases: [] }
			});
			const indicator = container.querySelector('[role="status"]');
			expect(indicator).toBeInTheDocument();
		});

		it('handles very long text', () => {
			const longText = 'A'.repeat(500);
			render(AnsweringIndicator, {
				status: { ...mockStatus, text: longText }
			});
			const indicator = screen.queryByText(longText);
			// Text should be truncated in display
			expect(screen.getByRole('status')).toBeInTheDocument();
		});
	});
});
