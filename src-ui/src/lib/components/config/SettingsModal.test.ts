import {fireEvent, render, screen} from '@testing-library/svelte';
import {describe, expect, it, vi} from 'vitest';
import SettingsModal from './SettingsModal.svelte';
import {configStore} from '$lib/stores/configStore';

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
				id: 'custom-provider',
				name: 'Custom Provider',
				icon: '🔌',
				isCustom: true,
				apiType: 'openai' as const,
				baseUrl: 'https://api.example.com',
				isConfigured: true
			}
		],
		operationDefaults: [
			{ operation: 'search', modelId: 'claude-3-opus' },
			{ operation: 'code', modelId: 'gpt-4' }
		],
		setOperationDefault: vi.fn(),
		removeProvider: vi.fn()
	},
	availableModelsByOperation: {
		search: [
			{
				id: 'claude-3-opus',
				name: 'Claude Opus',
				providerId: 'anthropic',
				capabilities: { vision: true, audio: false, streaming: true }
			}
		],
		code: [
			{
				id: 'gpt-4',
				name: 'GPT-4',
				providerId: 'openai',
				capabilities: { vision: true, audio: false, streaming: true }
			}
		],
		eye: [],
		mic: [],
		lightbulb: [],
		'check-circle': []
	}
}));

// Mock OPERATION_CONFIGS
vi.mock('$lib/types/config', () => ({
	OPERATION_CONFIGS: [
		{
			id: 'search',
			name: 'Search',
			description: 'Search and retrieve information',
			icon: 'search',
			color: '#5c6178'
		},
		{
			id: 'code',
			name: 'Code',
			description: 'Generate and analyze code',
			icon: 'code',
			color: '#2d9468'
		},
		{
			id: 'eye',
			name: 'Vision',
			description: 'Analyze images and visual content',
			icon: 'eye',
			color: '#f59e0b'
		},
		{
			id: 'mic',
			name: 'Audio',
			description: 'Process and generate audio',
			icon: 'mic',
			color: '#ef4444'
		},
		{
			id: 'lightbulb',
			name: 'Brainstorm',
			description: 'Generate ideas and solutions',
			icon: 'lightbulb',
			color: '#8b5cf6'
		},
		{
			id: 'check-circle',
			name: 'Review',
			description: 'Review and validate work',
			icon: 'check-circle',
			color: '#06b6d4'
		}
	]
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

describe('SettingsModal', () => {
	const mockProps = {
		open: true,
		onclose: vi.fn(),
		onconfigure: vi.fn(),
		onaddcustom: vi.fn()
	};

	describe('rendering when open', () => {
		it('renders modal with correct title', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Settings')).toBeInTheDocument();
		});
	});

	describe('Providers section', () => {
		it('renders providers section header', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Providers')).toBeInTheDocument();
		});

		it('renders "Add Provider" button in section header', () => {
			render(SettingsModal, mockProps);
			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			expect(addButton).toBeInTheDocument();
		});

		it('calls onaddcustom when Add Provider button is clicked', async () => {
			render(SettingsModal, mockProps);

			const addButton = screen.getByRole('button', { name: 'Add Provider' });
			await fireEvent.click(addButton);

			expect(mockProps.onaddcustom).toHaveBeenCalledTimes(1);
		});

		it('renders provider cards for each provider', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Anthropic')).toBeInTheDocument();
			expect(screen.getByText('OpenAI')).toBeInTheDocument();
			expect(screen.getByText('Custom Provider')).toBeInTheDocument();
		});

		it('shows "Connected" badge for configured providers', () => {
			render(SettingsModal, mockProps);
			expect(screen.getAllByText('Connected')).toHaveLength(3);
		});

		it('shows provider API type', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('anthropic')).toBeInTheDocument();
			expect(screen.getByText('openai')).toBeInTheDocument();
		});

		it('calls onconfigure when Edit button is clicked', async () => {
			render(SettingsModal, mockProps);

			const editButton = screen.getByRole('button', { name: 'Edit' });
			await fireEvent.click(editButton);

			// Should call with the first provider's ID
			expect(mockProps.onconfigure).toHaveBeenCalled();
		});

		it('shows Remove button only for custom providers', () => {
			render(SettingsModal, mockProps);

			// Should have one Remove button (for the custom provider)
			const removeButtons = screen.getAllByRole('button', { name: 'Remove' });
			expect(removeButtons).toHaveLength(1);
		});

		it('calls handleRemoveProvider when Remove is clicked', async () => {
			const removeProviderSpy = vi.fn();
			vi.mocked(configStore).removeProvider = removeProviderSpy;

			render(SettingsModal, mockProps);

			const removeButton = screen.getByRole('button', { name: 'Remove' });
			await fireEvent.click(removeButton);

			expect(removeProviderSpy).toHaveBeenCalledWith('custom-provider');
		});
	});

	describe('Models section', () => {
		it('renders models section header', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Default Models by Operation')).toBeInTheDocument();
		});

		it('renders operation groups for each operation', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Search')).toBeInTheDocument();
			expect(screen.getByText('Code')).toBeInTheDocument();
		});

		it('shows operation description', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Search and retrieve information')).toBeInTheDocument();
			expect(screen.getByText('Generate and analyze code')).toBeInTheDocument();
		});

		it('renders operation icons', () => {
			const { container } = render(SettingsModal, mockProps);
			const svgs = container.querySelectorAll('.operationIcon svg');
			expect(svgs.length).toBeGreaterThan(0);
		});

		it('shows selected model badge when model is selected', () => {
			render(SettingsModal, mockProps);
			// Should show "Opus" for search operation
			expect(screen.getByText('Opus')).toBeInTheDocument();
		});
	});

	describe('model selection', () => {
		it('renders model cards for available models', () => {
			render(SettingsModal, mockProps);
			expect(screen.getByText('Claude Opus')).toBeInTheDocument();
			expect(screen.getByText('GPT-4')).toBeInTheDocument();
		});

		it('calls setOperationDefault when model card is clicked', async () => {
			const setOperationDefaultSpy = vi.fn();
			vi.mocked(configStore).setOperationDefault = setOperationDefaultSpy;

			render(SettingsModal, mockProps);

			const modelCard = screen.getByRole('button', { name: /select claude opus/i });
			await fireEvent.click(modelCard);

			expect(setOperationDefaultSpy).toHaveBeenCalled();
		});
	});

	describe('empty states', () => {
		it('shows empty state when no providers exist', () => {
			vi.mocked(configStore).providers = [];

			render(SettingsModal, mockProps);

			expect(screen.getByText('No providers configured')).toBeInTheDocument();
		});

		it('shows empty models message when operation has no models', () => {
			render(SettingsModal, mockProps);

			// Vision operation should have no models
			const emptyMessages = screen.getAllByText('No models available');
			expect(emptyMessages.length).toBeGreaterThan(0);
		});
	});

	describe('styling and structure', () => {
		it('has proper section structure', () => {
			const { container } = render(SettingsModal, mockProps);

			const sections = container.querySelectorAll('.section');
			expect(sections.length).toBeGreaterThan(0);
		});

		it('renders operation groups with proper structure', () => {
			const { container } = render(SettingsModal, mockProps);

			const operationGroups = container.querySelectorAll('.operationGroup');
			expect(operationGroups.length).toBeGreaterThan(0);
		});
	});
});
