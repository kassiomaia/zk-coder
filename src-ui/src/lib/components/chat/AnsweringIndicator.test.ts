import { render, screen } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import AnsweringIndicator from './AnsweringIndicator.svelte';
import type { AgentStatus } from '$lib/utils/agentStatus';

describe('AnsweringIndicator', () => {
	const mockStatus: AgentStatus = {
		text: 'Analyzing code',
		active: true,
		icon: 'sparkle'
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
			expect(container.querySelector('[role="status"]')).toBeInTheDocument();
		});

		it('renders status text from status.text when no verb set', () => {
			render(AnsweringIndicator, { status: mockStatus });
			expect(screen.getByText('Analyzing code')).toBeInTheDocument();
		});

		it('renders verb when provided', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, verb: 'reading', target: 'src/main.rs' }
			});
			expect(screen.getByText('reading')).toBeInTheDocument();
			expect(screen.getByText('src/main.rs')).toBeInTheDocument();
		});

		it('renders waveform bars', () => {
			const { container } = render(AnsweringIndicator, { status: mockStatus });
			const bars = container.querySelectorAll('[class*="bar"]');
			expect(bars.length).toBeGreaterThan(0);
		});

		it('applies thinking state class for sparkle icon', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'sparkle' }
			});
			const el = container.querySelector('[class*="stateThinking"]');
			expect(el).toBeInTheDocument();
		});

		it('applies tool state class for file icon', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'file' }
			});
			const el = container.querySelector('[class*="stateTool"]');
			expect(el).toBeInTheDocument();
		});

		it('applies tool state class for terminal icon', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'terminal' }
			});
			const el = container.querySelector('[class*="stateTool"]');
			expect(el).toBeInTheDocument();
		});

		it('applies tool state class for edit icon', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, icon: 'edit' }
			});
			const el = container.querySelector('[class*="stateTool"]');
			expect(el).toBeInTheDocument();
		});

		it('applies complete state when inactive with check icon', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, active: false, icon: 'check' }
			});
			const el = container.querySelector('[class*="stateComplete"]');
			expect(el).toBeInTheDocument();
		});

		it('shows "done" text in complete state', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, active: false, icon: 'check' }
			});
			expect(screen.getByText('done')).toBeInTheDocument();
		});

		it('shows checkmark in complete state', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, active: false, icon: 'check' }
			});
			expect(screen.getByText('✓')).toBeInTheDocument();
		});
	});

	describe('stall warnings', () => {
		it('displays "still working" for any stall warning', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, stallWarning: 'still-working', elapsedSeconds: 35 }
			});
			expect(screen.getByText('still working')).toBeInTheDocument();
		});

		it('applies stall state class when stall warning present', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, stallWarning: 'taking-longer', elapsedSeconds: 90 }
			});
			expect(container.querySelector('[class*="stateStall"]')).toBeInTheDocument();
		});

		it('applies stall state for very-long warning', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, stallWarning: 'very-long', elapsedSeconds: 300 }
			});
			expect(container.querySelector('[class*="stateStall"]')).toBeInTheDocument();
		});

		it('hides target text when stall warning is active', () => {
			render(AnsweringIndicator, {
				status: {
					...mockStatus,
					stallWarning: 'still-working',
					verb: 'reading',
					target: 'src/main.rs'
				}
			});
			expect(screen.queryByText('src/main.rs')).not.toBeInTheDocument();
		});
	});

	describe('elapsed counter', () => {
		it('does not show elapsed when seconds < 2', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, elapsedSeconds: 1 }
			});
			expect(container.querySelector('[class*="elapsed"]')).not.toBeInTheDocument();
		});

		it('shows seconds format when < 60s', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, elapsedSeconds: 30 }
			});
			expect(screen.getByText('30s')).toBeInTheDocument();
		});

		it('shows minutes format when >= 60s', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, elapsedSeconds: 120 }
			});
			expect(screen.getByText('2m 00s')).toBeInTheDocument();
		});

		it('pads seconds in minutes format', () => {
			render(AnsweringIndicator, {
				status: { ...mockStatus, elapsedSeconds: 84 }
			});
			expect(screen.getByText('1m 24s')).toBeInTheDocument();
		});
	});

	describe('accessibility', () => {
		it('has role="status"', () => {
			const { container } = render(AnsweringIndicator, { status: mockStatus });
			expect(container.querySelector('[role="status"]')).toBeInTheDocument();
		});

		it('has aria-live="polite"', () => {
			const { container } = render(AnsweringIndicator, { status: mockStatus });
			expect(container.querySelector('[aria-live="polite"]')).toBeInTheDocument();
		});

		it('has descriptive aria-label', () => {
			const { container } = render(AnsweringIndicator, { status: mockStatus });
			expect(container.querySelector('[aria-label="Agent status"]')).toBeInTheDocument();
		});
	});

	describe('edge cases', () => {
		it('handles null status gracefully', () => {
			const { container } = render(AnsweringIndicator, { status: null });
			expect(container.querySelector('[role="status"]')).toBeInTheDocument();
		});

		it('handles undefined status gracefully', () => {
			const { container } = render(AnsweringIndicator, { status: undefined });
			expect(container.querySelector('[role="status"]')).toBeInTheDocument();
		});

		it('handles empty text', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, text: '' }
			});
			expect(container.querySelector('[role="status"]')).toBeInTheDocument();
		});

		it('shows thinking state for null status', () => {
			const { container } = render(AnsweringIndicator, { status: null });
			expect(container.querySelector('[class*="stateThinking"]')).toBeInTheDocument();
		});

		it('does not show target element when target is empty', () => {
			const { container } = render(AnsweringIndicator, {
				status: { ...mockStatus, verb: 'searching' }
			});
			const statusText = container.querySelector('[class*="statusText"]');
			const targets = statusText?.querySelectorAll('[class*="statusTarget"]');
			expect(targets?.length ?? 0).toBe(0);
		});
	});
});
