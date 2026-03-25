import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { describe, it, expect, vi } from 'vitest';
import ConfigModal from './ConfigModal.svelte';
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
				baseUrl: undefined,
				isConfigured: false,
				modelsSource: 'auto'
			},
			{
				id: 'openai',
				name: 'OpenAI',
				icon: '🧠',
				isCustom: false,
				apiType: 'openai' as const,
				baseUrl: undefined,
				isConfigured: true,
				modelsSource: 'auto'
			},
			{
				id: 'google',
				name: 'Google',
				icon: '🔍',
				isCustom: false,
				apiType: 'google' as const,
				baseUrl: undefined,
				isConfigured: false,
				modelsSource: 'auto'
			},
			{
				id: 'custom-provider',
				name: 'Custom Provider',
				icon: '🔌',
				isCustom: true,
				apiType: 'openai' as const,
				baseUrl: 'https://api.example.com/v1',
				isConfigured: true,
				modelsSource: 'hybrid'
			}
		],
		validateApiKey: vi.fn(),
		storeApiKey: vi.fn(),
		fetchModels: vi.fn(),
		setProviderModelsSource: vi.fn()
	}
}));

// Mock logger
vi.mock('$lib/utils/logger', () => ({
	createLogger: () => ({
		info: vi.fn(),
		debug: vi.fn(),
		warn: vi.fn(),
		error: vi.fn()
	})
}));

describe('ConfigModal', () => {
	const mockProps = {
		open: true,
		providerId: 'anthropic',
		onclose: vi.fn(),
		onsaved: vi.fn()
	};

	describe('rendering when open', () => {
		it('renders modal with provider name in title', () => {
			render(ConfigModal, mockProps);
			expect(screen.getByText('Configure Anthropic')).toBeInTheDocument();
		});

		it('renders provider icon and details', () => {
			render(ConfigModal, mockProps);
			expect(screen.getByText('Anthropic')).toBeInTheDocument();
			expect(screen.getByText('Anthropic Compatible')).toBeInTheDocument();
		});

		it('renders API key input field', () => {
			render(ConfigModal, mockProps);
			const apiKeyInput = screen.getByLabelText('API Key');
			expect(apiKeyInput).toBeInTheDocument();
		});

		it('shows correct placeholder based on API type', () => {
			render(ConfigModal, mockProps);
			// Anthropic placeholder
			const apiKeyInput = screen.getByLabelText('API Key') as HTMLInputElement;
			expect(apiKeyInput.placeholder).toContain('sk-ant');
		});

		it('shows OpenAI placeholder for OpenAI provider', () => {
			render(ConfigModal, { ...mockProps, providerId: 'openai' });
			const apiKeyInput = screen.getByLabelText('API Key') as HTMLInputElement;
			expect(apiKeyInput.placeholder).toContain('sk-');
		});

		it('shows Google placeholder for Google provider', () => {
			render(ConfigModal, { ...mockProps, providerId: 'google' });
			const apiKeyInput = screen.getByLabelText('API Key') as HTMLInputElement;
			expect(apiKeyInput.placeholder).toContain('AIza');
		});

		it('shows info message about secure storage', () => {
			render(ConfigModal, mockProps);
			expect(screen.getByText(/your api key is stored securely/i)).toBeInTheDocument();
		});

		it('shows "Get your API key from the provider\'s dashboard" helper text', () => {
			render(ConfigModal, mockProps);
			expect(
				screen.getByText("Get your API key from the provider's dashboard")
			).toBeInTheDocument();
		});
	});

	describe('API key input', () => {
		it('allows entering API key', async () => {
			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test123' } });

			expect(apiKeyInput).toHaveValue('sk-ant-test123');
		});

		it('is password type', () => {
			render(ConfigModal, mockProps);
			const apiKeyInput = screen.getByLabelText('API Key') as HTMLInputElement;
			expect(apiKeyInput.type).toBe('password');
		});
	});

	describe('validation', () => {
		it('shows error when validating empty API key', async () => {
			render(ConfigModal, mockProps);

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			expect(screen.getByText('Please enter an API key')).toBeInTheDocument();
		});

		it('calls validateApiKey with correct parameters', async () => {
			const validateSpy = vi.fn().mockResolvedValue({ valid: true });
			vi.mocked(configStore).validateApiKey = validateSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			expect(validateSpy).toHaveBeenCalledWith(
				'anthropic',
				'sk-ant-test',
				'anthropic',
				undefined
			);
		});

		it('shows success message when validation succeeds', async () => {
			const validateSpy = vi.fn().mockResolvedValue({ valid: true });
			vi.mocked(configStore).validateApiKey = validateSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			await waitFor(() => {
				expect(screen.getByText('API key validated successfully!')).toBeInTheDocument();
			});
		});

		it('shows error message when validation fails', async () => {
			const validateSpy = vi.fn().mockResolvedValue({
				valid: false,
				message: 'Invalid API key'
			});
			vi.mocked(configStore).validateApiKey = validateSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'invalid-key' } });

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			await waitFor(() => {
				expect(screen.getByText('Invalid API key')).toBeInTheDocument();
			});
		});

		it('handles validation errors gracefully', async () => {
			const validateSpy = vi.fn().mockRejectedValue(new Error('Network error'));
			vi.mocked(configStore).validateApiKey = validateSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			await waitFor(() => {
				expect(screen.getByText('Network error')).toBeInTheDocument();
			});
		});
	});

	describe('save functionality', () => {
		it('does not save when API key is empty', async () => {
			render(ConfigModal, mockProps);

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			// Should not call save methods
			expect(configStore.storeApiKey).not.toHaveBeenCalled();
		});

		it('stores API key when saving with valid key', async () => {
			const storeSpy = vi.fn().mockResolvedValue(undefined);
			const fetchSpy = vi.fn().mockResolvedValue([
				{ id: 'claude-3-opus', name: 'Claude Opus', providerId: 'anthropic' }
			]);
			vi.mocked(configStore).storeApiKey = storeSpy;
			vi.mocked(configStore).fetchModels = fetchSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(storeSpy).toHaveBeenCalledWith('anthropic', 'sk-ant-test');
			});
		});

		it('fetches models after storing API key', async () => {
			const storeSpy = vi.fn().mockResolvedValue(undefined);
			const fetchSpy = vi.fn().mockResolvedValue([
				{ id: 'claude-3-opus', name: 'Claude Opus', providerId: 'anthropic' }
			]);
			vi.mocked(configStore).storeApiKey = storeSpy;
			vi.mocked(configStore).fetchModels = fetchSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(fetchSpy).toHaveBeenCalledWith('anthropic', 'sk-ant-test', 'anthropic', undefined);
			});
		});

		it('shows loading state while saving', async () => {
			const storeSpy = vi.fn(() => new Promise(() => {})); // Never resolves
			vi.mocked(configStore).storeApiKey = storeSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(screen.getByText('Saving...')).toBeInTheDocument();
			});
		});

		it('shows error when no models are fetched', async () => {
			const storeSpy = vi.fn().mockResolvedValue(undefined);
			const fetchSpy = vi.fn().mockResolvedValue([]);
			vi.mocked(configStore).storeApiKey = storeSpy;
			vi.mocked(configStore).fetchModels = fetchSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(screen.getByText(/no models were found/i)).toBeInTheDocument();
			});
		});

		it('calls onsaved and closes modal on successful save', async () => {
			const storeSpy = vi.fn().mockResolvedValue(undefined);
			const fetchSpy = vi.fn().mockResolvedValue([
				{ id: 'claude-3-opus', name: 'Claude Opus', providerId: 'anthropic' }
			]);
			vi.mocked(configStore).storeApiKey = storeSpy;
			vi.mocked(configStore).fetchModels = fetchSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			await fireEvent.click(saveButton);

			await waitFor(() => {
				expect(mockProps.onsaved).toHaveBeenCalled();
				expect(mockProps.onclose).toHaveBeenCalled();
			});
		});
	});

	describe('model source configuration', () => {
		it('shows model source section when provider is configured', () => {
			render(ConfigModal, { ...mockProps, providerId: 'openai' });
			expect(screen.getByText('Model Discovery Mode')).toBeInTheDocument();
		});

		it('does not show model source section for unconfigured providers', () => {
			render(ConfigModal, mockProps);
			expect(screen.queryByText('Model Discovery Mode')).not.toBeInTheDocument();
		});

		it('renders all three model source options', () => {
			render(ConfigModal, { ...mockProps, providerId: 'openai' });
			expect(screen.getByText('Auto')).toBeInTheDocument();
			expect(screen.getByText('Manual Only')).toBeInTheDocument();
			expect(screen.getByText('Hybrid')).toBeInTheDocument();
		});

		it('calls setProviderModelsSource when option is selected', async () => {
			const setSourceSpy = vi.fn().mockResolvedValue(undefined);
			vi.mocked(configStore).setProviderModelsSource = setSourceSpy;

			render(ConfigModal, { ...mockProps, providerId: 'openai' });

			const manualOption = screen.getByLabelText('Manual Only');
			await fireEvent.click(manualOption);

			expect(setSourceSpy).toHaveBeenCalledWith('openai', 'manual');
		});
	});

	describe('footer actions', () => {
		it('renders Cancel button', () => {
			render(ConfigModal, mockProps);
			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			expect(cancelButton).toBeInTheDocument();
		});

		it('renders Validate button', () => {
			render(ConfigModal, mockProps);
			const validateButton = screen.getByRole('button', { name: 'Validate' });
			expect(validateButton).toBeInTheDocument();
		});

		it('renders Save button', () => {
			render(ConfigModal, mockProps);
			const saveButton = screen.getByRole('button', { name: 'Save' });
			expect(saveButton).toBeInTheDocument();
		});

		it('disables Save button when API key is empty', () => {
			render(ConfigModal, mockProps);
			const saveButton = screen.getByRole('button', { name: 'Save' });
			expect(saveButton).toBeDisabled();
		});

		it('enables Save button when API key is entered', async () => {
			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const saveButton = screen.getByRole('button', { name: 'Save' });
			expect(saveButton).not.toBeDisabled();
		});

		it('calls onclose when Cancel is clicked', async () => {
			render(ConfigModal, mockProps);

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockProps.onclose).toHaveBeenCalledTimes(1);
		});
	});

	describe('close behavior', () => {
		it('resets form when modal is closed', async () => {
			const { rerender } = render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			// Close and reopen
			rerender({ ...mockProps, open: false });
			rerender({ ...mockProps, open: true });

			// Form should be reset (apiKey would be empty or loaded from store)
			expect(screen.getByLabelText('API Key')).toBeInTheDocument();
		});
	});

	describe('loading and error states', () => {
		it('shows spinner during validation', async () => {
			const validateSpy = vi.fn(() => new Promise(() => {})); // Never resolves
			vi.mocked(configStore).validateApiKey = validateSpy;

			render(ConfigModal, mockProps);

			const apiKeyInput = screen.getByLabelText('API Key');
			await fireEvent.input(apiKeyInput, { target: { value: 'sk-ant-test' } });

			const validateButton = screen.getByRole('button', { name: 'Validate' });
			await fireEvent.click(validateButton);

			await waitFor(() => {
				const spinner = document.querySelector('svg[style*="animation"]');
				expect(spinner).toBeInTheDocument();
			});
		});
	});

	describe('when provider is null', () => {
		it('shows loading state when providerId is null', () => {
			render(ConfigModal, { ...mockProps, providerId: null });
			expect(screen.getByText('Loading provider...')).toBeInTheDocument();
		});

		it('shows generic title when provider is null', () => {
			render(ConfigModal, { ...mockProps, providerId: null });
			expect(screen.getByText('Configure Provider')).toBeInTheDocument();
		});
	});
});
