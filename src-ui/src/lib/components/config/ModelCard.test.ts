import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
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
			// Check icon is an SVG inside the button
			const checkIcon = button.querySelector('svg');
			expect(checkIcon).toBeInTheDocument();
		});

		it('does not show check icon when not selected', () => {
			render(ModelCard, { model: mockModel, selected: false });
			const button = screen.getByRole('button');
			// Only capability icons might be present, but no check icon
			const checkIcon = button.querySelector('.checkIcon');
			expect(checkIcon).not.toBeInTheDocument();
		});

		it('applies selected class when selected', () => {
			const { container } = render(ModelCard, {
				model: mockModel,
				selected: true
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('selected');
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
		it('applies card base class', () => {
			const { container } = render(ModelCard, { model: mockModel });
			const button = container.querySelector('button');
			expect(button).toHaveClass('card');
		});

		it('includes modelId class with model id', () => {
			const { container } = render(ModelCard, { model: mockModel });
			const modelId = container.querySelector('.modelId');
			expect(modelId).toBeInTheDocument();
		});

		it('includes modelName class with model name', () => {
			const { container } = render(ModelCard, { model: mockModel });
			const modelName = container.querySelector('.modelName');
			expect(modelName).toBeInTheDocument();
		});
	});
});
