import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ModelCard from './ModelCard.svelte';
import type { Model } from '$lib/types/config';

describe('ModelCard', () => {
	const mockModel: Model = {
		id: 'claude-3-opus-20240229',
		name: 'Claude Opus',
		providerId: 'anthropic',
		capabilities: {
			vision: true,
			audio: false,
			streaming: true
		}
	};

	const mockModelWithAllCapabilities: Model = {
		id: 'gpt-4-vision',
		name: 'GPT-4 Vision',
		providerId: 'openai',
		capabilities: {
			vision: true,
			audio: true,
			streaming: true
		}
	};

	const mockModelWithNoCapabilities: Model = {
		id: 'basic-model',
		name: 'Basic Model',
		providerId: 'custom',
		capabilities: {
			vision: false,
			audio: false,
			streaming: false
		}
	};

	describe('rendering', () => {
		it('renders model name and id', () => {
			render(ModelCard, { model: mockModel });

			expect(screen.getByText('Claude Opus')).toBeInTheDocument();
			expect(screen.getByText('claude-3-opus-20240229')).toBeInTheDocument();
		});

		it('renders as a button element', () => {
			render(ModelCard, { model: mockModel });
			const button = screen.getByRole('button', { name: /select claude opus/i });
			expect(button).toBeInTheDocument();
		});

		it('has correct aria-pressed state when not selected', () => {
			render(ModelCard, { model: mockModel, selected: false });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-pressed', 'false');
		});

		it('has correct aria-pressed state when selected', () => {
			render(ModelCard, { model: mockModel, selected: true });
			const button = screen.getByRole('button');
			expect(button).toHaveAttribute('aria-pressed', 'true');
		});
	});

	describe('selected state', () => {
		it('shows check icon when selected', () => {
			render(ModelCard, { model: mockModel, selected: true });
			const button = screen.getByRole('button');
			// Check icon is an SVG inside the button (the check icon has 16x16 size)
			const checkIcon = button.querySelector('svg[width="16"]');
			expect(checkIcon).toBeInTheDocument();
		});

		it('does not show check icon when not selected', () => {
			render(ModelCard, { model: mockModel, selected: false });
			const button = screen.getByRole('button');
			// No 16x16 check icon when not selected (capability icons are 12x12)
			const checkIcon = button.querySelector('svg[width="16"]');
			expect(checkIcon).not.toBeInTheDocument();
		});

		it('applies selected state via aria-pressed', () => {
			const { container } = render(ModelCard, {
				model: mockModel,
				selected: true
			});
			const button = container.querySelector('button');
			expect(button).toHaveAttribute('aria-pressed', 'true');
		});
	});

	describe('capabilities', () => {
		it('shows vision capability badge when model supports vision', () => {
			render(ModelCard, { model: mockModel });
			const visionBadge = screen.getByTitle('Supports vision/images');
			expect(visionBadge).toBeInTheDocument();
		});

		it('shows streaming capability badge when model supports streaming', () => {
			render(ModelCard, { model: mockModel });
			const streamingBadge = screen.getByTitle('Supports streaming');
			expect(streamingBadge).toBeInTheDocument();
		});

		it('shows audio capability badge when model supports audio', () => {
			render(ModelCard, { model: mockModelWithAllCapabilities });
			const audioBadge = screen.getByTitle('Supports audio');
			expect(audioBadge).toBeInTheDocument();
		});

		it('does not show capability badges when model has no capabilities', () => {
			const { container } = render(ModelCard, {
				model: mockModelWithNoCapabilities
			});
			const capabilities = container.querySelector('.capabilities');
			expect(capabilities).not.toBeInTheDocument();
		});

		it('shows all capability badges for model with all capabilities', () => {
			const { container } = render(ModelCard, {
				model: mockModelWithAllCapabilities
			});
			const capabilities = container.querySelectorAll('.capability');
			expect(capabilities).toHaveLength(3);
		});
	});

	describe('interactions', () => {
		it('calls onclick handler when clicked', async () => {
			const handleClick = vi.fn();
			render(ModelCard, { model: mockModel, onclick: handleClick });

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).toHaveBeenCalledTimes(1);
		});

		it('does not call onclick when not provided', async () => {
			// Should not throw error
			render(ModelCard, { model: mockModel });

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			// If we got here, no error was thrown
			expect(button).toBeInTheDocument();
		});
	});

	describe('styling classes', () => {
		it('renders as button element', () => {
			const { container } = render(ModelCard, { model: mockModel });
			const button = container.querySelector('button');
			expect(button).toBeInTheDocument();
		});

		it('includes modelId text content', () => {
			const { container } = render(ModelCard, { model: mockModel });
			// The model ID should be visible in the component
			expect(container.textContent).toContain('claude-3-opus-20240229');
		});

		it('includes modelName text content', () => {
			const { container } = render(ModelCard, { model: mockModel });
			// The model name should be visible in the component
			expect(container.textContent).toContain('Claude Opus');
		});
	});
});
