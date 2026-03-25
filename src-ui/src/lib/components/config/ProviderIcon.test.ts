import { render } from '@testing-library/svelte';
import { describe, it, expect } from 'vitest';
import ProviderIcon from './ProviderIcon.svelte';
import type { Provider } from '$lib/types/config';

describe('ProviderIcon', () => {
	const mockProviders: Record<string, Provider> = {
		anthropic: {
			id: 'anthropic',
			name: 'Anthropic',
			icon: '🤖',
			isCustom: false,
			apiType: 'anthropic',
			isConfigured: false
		},
		openai: {
			id: 'openai',
			name: 'OpenAI',
			icon: '🧠',
			isCustom: false,
			apiType: 'openai',
			isConfigured: false
		},
		google: {
			id: 'google',
			name: 'Google',
			icon: '🔍',
			isCustom: false,
			apiType: 'google',
			isConfigured: false
		},
		custom: {
			id: 'custom-provider',
			name: 'Custom Provider',
			icon: '🔌',
			isCustom: true,
			apiType: 'openai',
			baseUrl: 'https://api.example.com',
			isConfigured: false
		}
	};

	describe('Anthropic provider', () => {
		it('renders anthropic icon SVG', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.anthropic
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('has correct paths for anthropic (chip-like icon)', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.anthropic
			});
			const paths = container.querySelectorAll('path');
			const rect = container.querySelector('rect');
			expect(paths.length).toBeGreaterThan(0);
			expect(rect).toBeInTheDocument();
		});

		it('uses CSS variable for anthropic color', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.anthropic
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('stroke')).toBe('var(--color-provider-anthropic)');
		});
	});

	describe('OpenAI provider', () => {
		it('renders openai icon SVG', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('has rects for openai (chip-like icon)', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai
			});
			const rects = container.querySelectorAll('rect');
			const paths = container.querySelectorAll('path');
			expect(rects.length).toBeGreaterThan(0);
			expect(paths.length).toBeGreaterThan(0);
		});

		it('uses CSS variable for openai color', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('stroke')).toBe('var(--color-provider-openai)');
		});
	});

	describe('Google provider', () => {
		it('renders google icon SVG', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.google
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('has circle and paths for google (globe-like icon)', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.google
			});
			const circle = container.querySelector('circle');
			const paths = container.querySelectorAll('path');
			expect(circle).toBeInTheDocument();
			expect(paths.length).toBeGreaterThan(0);
		});

		it('uses CSS variable for google color', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.google
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('stroke')).toBe('var(--color-provider-google)');
		});
	});

	describe('Custom/unknown provider', () => {
		it('renders default icon for unknown providers', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.custom
			});
			const svg = container.querySelector('svg');
			expect(svg).toBeInTheDocument();
		});

		it('uses default CSS variable color for unknown providers', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.custom
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('stroke')).toBe('var(--color-provider-default)');
		});
	});

	describe('Size prop', () => {
		it('renders with default size of 32', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('width')).toBe('32');
			expect(svg.getAttribute('height')).toBe('32');
		});

		it('renders with custom size', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai,
				size: 48
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('width')).toBe('48');
			expect(svg.getAttribute('height')).toBe('48');
		});

		it('renders with size 24', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai,
				size: 24
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('width')).toBe('24');
			expect(svg.getAttribute('height')).toBe('24');
		});
	});

	describe('SVG attributes', () => {
		it('all icons have correct base SVG attributes', () => {
			const { container } = render(ProviderIcon, {
				provider: mockProviders.openai
			});
			const svg = container.querySelector('svg') as SVGElement;
			expect(svg.getAttribute('fill')).toBe('none');
			expect(svg.getAttribute('stroke-width')).toBe('2');
			expect(svg.getAttribute('stroke-linecap')).toBe('round');
			expect(svg.getAttribute('stroke-linejoin')).toBe('round');
		});
	});
});
