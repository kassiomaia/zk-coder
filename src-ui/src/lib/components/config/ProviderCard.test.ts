import { render, screen, fireEvent } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import ProviderCard from './ProviderCard.svelte';
import type { Provider } from '$lib/types/config';

describe('ProviderCard', () => {
	const mockUnconfiguredProvider: Provider = {
		id: 'anthropic',
		name: 'Anthropic',
		icon: '🤖',
		isCustom: false,
		apiType: 'anthropic',
		isConfigured: false
	};

	const mockConfiguredProvider: Provider = {
		id: 'openai',
		name: 'OpenAI',
		icon: '🧠',
		isCustom: false,
		apiType: 'openai',
		isConfigured: true
	};

	const mockCustomProvider: Provider = {
		id: 'my-custom-provider',
		name: 'My Custom Provider',
		icon: '🔌',
		isCustom: true,
		apiType: 'openai',
		baseUrl: 'https://api.example.com/v1',
		isConfigured: false
	};

	const mockConfiguredCustomProvider: Provider = {
		...mockCustomProvider,
		isConfigured: true
	};

	describe('rendering', () => {
		it('renders provider name', () => {
			render(ProviderCard, { provider: mockUnconfiguredProvider, onclick: vi.fn() });
			expect(screen.getByText('Anthropic')).toBeInTheDocument();
		});

		it('renders as a button element', () => {
			render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const button = screen.getByRole('button', { name: /configure anthropic/i });
			expect(button).toBeInTheDocument();
		});

		it('has correct aria-label', () => {
			render(ProviderCard, { provider: mockUnconfiguredProvider, onclick: vi.fn() });
			const button = screen.getByRole('button', { name: 'Configure Anthropic' });
			expect(button).toBeInTheDocument();
		});

		it('renders provider icon component', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			// ProviderIcon renders an SVG
			const svg = container.querySelector('.iconWrapper svg');
			expect(svg).toBeInTheDocument();
		});

		it('shows correct models text for built-in providers', () => {
			render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			// getProviderDefaultModels returns something like "Claude Models"
			const modelsText = screen.getByText(/claude Models/i);
			expect(modelsText).toBeInTheDocument();
		});

		it('shows base URL for custom providers', () => {
			render(ProviderCard, { provider: mockCustomProvider, onclick: vi.fn() });
			expect(screen.getByText('https://api.example.com/v1')).toBeInTheDocument();
		});
	});

	describe('unconfigured state', () => {
		it('shows "Click to configure" text when not configured', () => {
			render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			expect(screen.getByText('Click to configure')).toBeInTheDocument();
		});

		it('shows unstyled dot for unconfigured providers', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const dot = container.querySelector('.dot:not(.configured)');
			expect(dot).toBeInTheDocument();
		});

		it('applies card class without configured modifier', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('card');
			expect(button).not.toHaveClass('configured');
		});
	});

	describe('configured state', () => {
		it('shows "Configured" text when configured', () => {
			render(ProviderCard, {
				provider: mockConfiguredProvider,
				onclick: vi.fn()
			});
			expect(screen.getByText('Configured')).toBeInTheDocument();
		});

		it('shows green configured dot when configured', () => {
			const { container } = render(ProviderCard, {
				provider: mockConfiguredProvider,
				onclick: vi.fn()
			});
			const dot = container.querySelector('.dot.configured');
			expect(dot).toBeInTheDocument();
		});

		it('shows checkmark icon in configured state', () => {
			const { container } = render(ProviderCard, {
				provider: mockConfiguredProvider,
				onclick: vi.fn()
			});
			const checkmark = container.querySelector('.configuredText svg');
			expect(checkmark).toBeInTheDocument();
		});

		it('applies configured class when provider is configured', () => {
			const { container } = render(ProviderCard, {
				provider: mockConfiguredProvider,
				onclick: vi.fn()
			});
			const button = container.querySelector('button');
			expect(button).toHaveClass('configured');
		});
	});

	describe('custom providers', () => {
		it('displays base URL when provider is custom', () => {
			render(ProviderCard, { provider: mockCustomProvider, onclick: vi.fn() });
			expect(screen.getByText('https://api.example.com/v1')).toBeInTheDocument();
		});

		it('shows configured status for custom providers', () => {
			render(ProviderCard, {
				provider: mockConfiguredCustomProvider,
				onclick: vi.fn()
			});
			expect(screen.getByText('Configured')).toBeInTheDocument();
		});
	});

	describe('interactions', () => {
		it('calls onclick handler when clicked', async () => {
			const handleClick = vi.fn();
			render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: handleClick
			});

			const button = screen.getByRole('button');
			await fireEvent.click(button);

			expect(handleClick).toHaveBeenCalledTimes(1);
		});

		it('requires onclick prop to be provided', () => {
			// Component should render without error when onclick is provided
			expect(() => {
				render(ProviderCard, {
					provider: mockUnconfiguredProvider,
					onclick: vi.fn()
				});
			}).not.toThrow();
		});
	});

	describe('styling', () => {
		it('has iconWrapper class for icon container', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const iconWrapper = container.querySelector('.iconWrapper');
			expect(iconWrapper).toBeInTheDocument();
		});

		it('has name class for provider name', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const name = container.querySelector('.name');
			expect(name).toBeInTheDocument();
		});

		it('has models class for models text', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const models = container.querySelector('.models');
			expect(models).toBeInTheDocument();
		});

		it('has status class for status section', () => {
			const { container } = render(ProviderCard, {
				provider: mockUnconfiguredProvider,
				onclick: vi.fn()
			});
			const status = container.querySelector('.status');
			expect(status).toBeInTheDocument();
		});
	});
});
