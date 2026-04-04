import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import CustomProviderModal from './CustomProviderModal.svelte';
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
				isConfigured: true
			}
		],
		saveProvider: vi.fn()
	}
}));

// Mock svelte/store
vi.mock('svelte/store', () => ({
	get: vi.fn((store) => store)
}));

describe('CustomProviderModal', () => {
	const mockProps = {
		open: true,
		onclose: vi.fn(),
		onsaved: vi.fn()
	};

	describe('rendering when open', () => {
		it('renders modal with correct title', () => {
			render(CustomProviderModal, mockProps);
			expect(screen.getByText('Add Custom Provider')).toBeInTheDocument();
		});

		it('renders provider name input field', () => {
			render(CustomProviderModal, mockProps);
			const nameInput = screen.getByLabelText('Provider Name');
			expect(nameInput).toBeInTheDocument();
		});

		it('renders provider ID input field', () => {
			render(CustomProviderModal, mockProps);
			const idInput = screen.getByLabelText('Provider ID');
			expect(idInput).toBeInTheDocument();
		});

		it('renders base URL input field', () => {
			render(CustomProviderModal, mockProps);
			const urlInput = screen.getByLabelText('Base URL');
			expect(urlInput).toBeInTheDocument();
		});

		it('renders API type radio options', () => {
			render(CustomProviderModal, mockProps);
			expect(screen.getByText('OpenAI Compatible')).toBeInTheDocument();
			expect(screen.getByText('Anthropic Compatible')).toBeInTheDocument();
			expect(screen.getByText('Google (Gemini) Compatible')).toBeInTheDocument();
		});

		it('has OpenAI Compatible selected by default', () => {
			render(CustomProviderModal, mockProps);
			const openaiRadio = screen.getByLabelText('OpenAI Compatible');
			expect(openaiRadio).toBeChecked();
		});
	});

	describe('form interactions', () => {
		it('auto-generates ID from provider name', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Test Provider' } });

			// ID should be auto-generated from name
			const idInput = screen.getByLabelText('Provider ID') as HTMLInputElement;
			expect(idInput.value).toContain('my-test-provider');
		});

		it('allows manual ID editing', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Provider' } });

			const idInput = screen.getByLabelText('Provider ID');
			await fireEvent.input(idInput, { target: { value: 'my-custom-id' } });

			// Change name again, ID should not change after manual edit
			await fireEvent.input(nameInput, { target: { value: 'Different Name' } });

			const idInputAfter = screen.getByLabelText('Provider ID') as HTMLInputElement;
			expect(idInputAfter.value).toBe('my-custom-id');
		});

		it('switches API type when radio is clicked', async () => {
			render(CustomProviderModal, mockProps);

			const anthropicRadio = screen.getByLabelText('Anthropic Compatible');
			await fireEvent.click(anthropicRadio);

			expect(anthropicRadio).toBeChecked();
		});

		it('allows changing base URL', async () => {
			render(CustomProviderModal, mockProps);

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			expect(urlInput).toHaveValue('https://api.example.com/v1');
		});
	});

	describe('form validation', () => {
		it('shows error when provider name is empty on submit', async () => {
			render(CustomProviderModal, mockProps);

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			// Should show validation error
			expect(screen.getByText('Provider name is required')).toBeInTheDocument();
		});

		it('shows error when provider ID is empty on submit', async () => {
			render(CustomProviderModal, mockProps);

			// Enter name but clear ID
			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'Test' } });

			const idInput = screen.getByLabelText('Provider ID');
			await fireEvent.input(idInput, { target: { value: '' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(screen.getByText('Provider ID is required')).toBeInTheDocument();
		});

		it('shows error when base URL is empty on submit', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'Test Provider' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(screen.getByText('Base URL is required')).toBeInTheDocument();
		});

		it('shows error for invalid URL format', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'Test' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'not-a-valid-url' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(screen.getByText('Invalid URL format')).toBeInTheDocument();
		});

		it('shows error when provider ID already exists', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'Anthropic' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(screen.getByText('A provider with this ID already exists')).toBeInTheDocument();
		});

		it('disables submit button when form is incomplete', () => {
			render(CustomProviderModal, mockProps);

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			expect(addButton).toBeDisabled();
		});

		it('enables submit button when form is complete', async () => {
			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Provider' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			expect(addButton).not.toBeDisabled();
		});
	});

	describe('form submission', () => {
		it('calls configStore.saveProvider with correct data', async () => {
			const saveProviderSpy = vi.fn();
			vi.mocked(configStore).saveProvider = saveProviderSpy;

			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Custom Provider' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(saveProviderSpy).toHaveBeenCalledWith(
				expect.objectContaining({
					name: 'My Custom Provider',
					baseUrl: 'https://api.example.com/v1',
					isCustom: true,
					apiType: 'openai'
				})
			);
		});

		it('calls onsaved with provider ID after successful save', async () => {
			const saveProviderSpy = vi.fn(() => Promise.resolve());
			vi.mocked(configStore).saveProvider = saveProviderSpy;

			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Provider' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			await waitFor(() => {
				expect(mockProps.onsaved).toHaveBeenCalled();
			});
		});

		it('calls onclose after successful save', async () => {
			const saveProviderSpy = vi.fn(() => Promise.resolve());
			vi.mocked(configStore).saveProvider = saveProviderSpy;

			render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'My Provider' } });

			const urlInput = screen.getByLabelText('Base URL');
			await fireEvent.input(urlInput, { target: { value: 'https://api.example.com/v1' } });

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			await waitFor(() => {
				expect(mockProps.onclose).toHaveBeenCalled();
			});
		});
	});

	describe('footer actions', () => {
		it('renders Cancel button', () => {
			render(CustomProviderModal, mockProps);
			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			expect(cancelButton).toBeInTheDocument();
		});

		it('calls onclose when Cancel is clicked', async () => {
			render(CustomProviderModal, mockProps);

			const cancelButton = screen.getByRole('button', { name: 'Cancel' });
			await fireEvent.click(cancelButton);

			expect(mockProps.onclose).toHaveBeenCalledTimes(1);
		});
	});

	describe('info text', () => {
		it('shows info message about API key configuration', () => {
			render(CustomProviderModal, mockProps);
			expect(
				screen.getByText(/after adding, you'll need to configure the api key/i)
			).toBeInTheDocument();
		});
	});

	describe('reset on close', () => {
		it('resets form when modal is closed and reopened', async () => {
			const { rerender } = render(CustomProviderModal, mockProps);

			const nameInput = screen.getByLabelText('Provider Name');
			await fireEvent.input(nameInput, { target: { value: 'Test Provider' } });

			// Close modal
			await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));

			// Reopen modal
			rerender({ ...mockProps, open: true });

			const nameInputAfter = screen.getByLabelText('Provider Name') as HTMLInputElement;
			expect(nameInputAfter.value).toBe('');
		});
	});
});
