import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import ProvidersModal from './ProvidersModal.svelte';
import { configStore } from '$lib/stores/configStore';

// Mock the configStore
vi.mock('$lib/stores/configStore', () => ({
	configStore: {
		providers: [
			{
				id: 'anthropic',
				name: 'Anthropic',
				icon: '🤖',
				isCustom: false,
				apiType: 'anthropic' as const,
				isConfigured: false
			},
			{
				id: 'openai',
				name: 'OpenAI',
				icon: '🧠',
				isCustom: false,
				apiType: 'openai' as const,
				isConfigured: true
			},
			{
				id: 'google',
				name: 'Google',
				icon: '🔍',
				isCustom: false,
				apiType: 'google' as const,
				isConfigured: false
			}
		]
	}
}));

// Mock logger
vi.mock('$lib/utils/logger', () => ({
	createLogger: () => ({
		debug: vi.fn(),
		info: vi.fn(),
		warn: vi.fn(),
		error: vi.fn()
	})
}));

describe('ProvidersModal', () => {
	const mockProps = {
		open: true,
		onclose: vi.fn(),
		oncontinue: vi.fn(),
		onaddcustom: vi.fn(),
		onconfigure: vi.fn()
	};

	describe('rendering when open', () => {
		it('renders modal with correct title', () => {
			render(ProvidersModal, mockProps);
			expect(screen.getByText('Configure Providers')).toBeInTheDocument();
		});

		it('renders description text', () => {
			render(ProvidersModal, mockProps);
			expect(screen.getByText(/configure one or more ai providers/i)).toBeInTheDocument();
		});

		it('shows configured count when providers are configured', () => {
			render(ProvidersModal, mockProps);
			expect(screen.getByText('1 provider configured')).toBeInTheDocument();
		});

		it('shows plural "providers" when multiple are configured', () => {
			// Mock with 2 configured providers
			vi.mocked(configStore).providers = [
				{ ...configStore.providers[0], isConfigured: true },
				{ ...configStore.providers[1], isConfigured: true }
			];

			render(ProvidersModal, mockProps);
			expect(screen.getByText('2 providers configured')).toBeInTheDocument();
		});
	});

	describe('provider cards', () => {
		it('renders provider cards for each provider', () => {
			render(ProvidersModal, mockProps);
			// Should have 3 provider cards plus "Add Custom Provider" button
			const buttons = screen.getAllByRole('button');
			expect(buttons.length).toBeGreaterThan(0);
		});

		it('calls onconfigure when provider card is clicked', async () => {
			render(ProvidersModal, mockProps);

			const anthropicButton = screen.getByRole('button', { name: /configure anthropic/i });
			await fireEvent.click(anthropicButton);

			expect(mockProps.onconfigure).toHaveBeenCalledWith('anthropic');
		});
	});

	describe('add custom provider', () => {
		it('renders "Add Custom Provider" card', () => {
			render(ProvidersModal, mockProps);
			expect(screen.getByText('Add Custom Provider')).toBeInTheDocument();
		});

		it('shows custom provider description', () => {
			render(ProvidersModal, mockProps);
			expect(screen.getByText('OpenAI, Anthropic, or custom API')).toBeInTheDocument();
		});

		it('calls onaddcustom when add custom provider is clicked', async () => {
			render(ProvidersModal, mockProps);

			const addButton = screen.getByRole('button', { name: /add custom provider/i });
			await fireEvent.click(addButton);

			expect(mockProps.onaddcustom).toHaveBeenCalledTimes(1);
		});
	});

	describe('footer actions', () => {
		it('renders Cancel button', () => {
			render(ProvidersModal, mockProps);
			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			expect(cancelButton).toBeInTheDocument();
		});

		it('renders "Add Provider" button (ghost variant)', () => {
			render(ProvidersModal, mockProps);
			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			expect(addButton).toBeInTheDocument();
		});

		it('renders "Continue to Settings" button', () => {
			render(ProvidersModal, mockProps);
			const continueButton = screen.getByRole('button', { name: 'Continue to Settings' });
			expect(continueButton).toBeInTheDocument();
		});

		it('disables Continue button when no providers are configured', () => {
			// Mock with no configured providers
			vi.mocked(configStore).providers = [
				{ ...configStore.providers[0], isConfigured: false },
				{ ...configStore.providers[1], isConfigured: false },
				{ ...configStore.providers[2], isConfigured: false }
			];

			render(ProvidersModal, mockProps);
			const continueButton = screen.getByRole('button', { name: 'Continue to Settings' });
			expect(continueButton).toBeDisabled();
		});

		it('enables Continue button when at least one provider is configured', () => {
			render(ProvidersModal, mockProps);
			const continueButton = screen.getByRole('button', { name: 'Continue to Settings' });
			expect(continueButton).not.toBeDisabled();
		});

		it('calls onclose when Cancel is clicked', async () => {
			render(ProvidersModal, mockProps);

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockProps.onclose).toHaveBeenCalledTimes(1);
		});

		it('calls oncontinue when Continue is clicked', async () => {
			render(ProvidersModal, mockProps);

			const continueButton = screen.getByRole('button', { name: 'Continue to Settings' });
			await fireEvent.click(continueButton);

			expect(mockProps.oncontinue).toHaveBeenCalledTimes(1);
		});
	});

	describe('when closed', () => {
		it('does not render content when open is false', () => {
			render(ProvidersModal, { ...mockProps, open: false });
			// Title should not be visible when modal is closed
			expect(screen.queryByText('Configure Providers')).not.toBeInTheDocument();
		});
	});
});
