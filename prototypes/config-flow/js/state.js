/**
 * LLM Configuration Flow - Alpine.js State Management
 * All reactive state and logic for the configuration flow prototype.
 */

function configFlow() {
    return {
        // Flow state
        currentFlow: 'welcome',
        flowIndex: 0,
        settingsOpen: false,
        configOpen: false,
        customProviderOpen: false,

        // Custom provider form
        customProvider: {
            name: '',
            apiType: '',
            baseUrl: '',
            modelsInput: ''
        },

        // Providers data
        providersList: [
            { id: 'anthropic', name: 'Anthropic', icon: 'AC', iconClass: 'anthropic', models: ['Claude Opus', 'Claude Sonnet', 'Claude Haiku'], configured: false, apiKey: '', isCustom: false },
            { id: 'openai', name: 'OpenAI', icon: 'OA', iconClass: 'openai', models: ['GPT-4 Turbo', 'GPT-4'], configured: false, apiKey: '', isCustom: false },
            { id: 'google', name: 'Google', icon: 'GC', iconClass: 'google', models: ['Gemini Pro', 'Gemini Ultra'], configured: false, apiKey: '', isCustom: false }
        ],

        // Current provider being configured
        currentProvider: null,

        // Validation
        validationMessage: '',
        validationClass: '',
        validationIcon: '',

        selectedModels: {
            analysis: null,
            code: null,
            vision: null,
            audio: null,
            tips: null,
            review: null
        },

        // Model tabs with Lucide icon names
        modelTabs: [
            {
                id: 'analysis',
                name: 'Analysis',
                label: 'Analysis',
                description: 'Simple models for quick questions',
                color: 'var(--color-info)',
                icon: 'search',
                iconLarge: 'search'
            },
            {
                id: 'code',
                name: 'Code Generation',
                label: 'Code Gen',
                description: 'Powerful models for generating code',
                color: 'var(--color-warning)',
                icon: 'code',
                iconLarge: 'code'
            },
            {
                id: 'vision',
                name: 'Images & Vision',
                label: 'Vision',
                description: 'Models with vision capability',
                color: 'var(--color-primary-500)',
                icon: 'eye',
                iconLarge: 'eye'
            },
            {
                id: 'audio',
                name: 'Audio & Transcription',
                label: 'Audio',
                description: 'Audio recording and transcription models',
                color: 'var(--color-success)',
                icon: 'mic',
                iconLarge: 'mic'
            },
            {
                id: 'tips',
                name: 'Tips & Refinements',
                label: 'Tips',
                description: 'Quick suggestions and improvements',
                color: 'var(--color-warning)',
                icon: 'lightbulb',
                iconLarge: 'lightbulb'
            },
            {
                id: 'review',
                name: 'Code Review',
                label: 'Review',
                description: 'Quality & best practices',
                color: 'var(--color-primary-500)',
                icon: 'check-circle',
                iconLarge: 'check-circle'
            }
        ],

        // Computed
        get configuredCount() {
            return this.providersList.filter(p => p.configured).length;
        },

        // Display model info
        get displayModelProvider() {
            const modelId = this.selectedModels.code || this.selectedModels.audio || this.selectedModels.analysis || this.selectedModels.vision;
            if (!modelId) return 'anthropic';
            return modelId.split('-')[0];
        },

        get displayModelName() {
            const modelId = this.selectedModels.code || this.selectedModels.audio || this.selectedModels.analysis || this.selectedModels.vision;
            if (!modelId) return 'No model selected';

            const [, ...modelParts] = modelId.split('-');
            const nameMap = {
                'claude-opus': 'Claude Opus',
                'claude-sonnet': 'Claude Sonnet',
                'claude-haiku': 'Claude Haiku',
                'gpt-4-turbo': 'GPT-4 Turbo',
                'gpt-4': 'GPT-4',
                'gemini-ultra': 'Gemini Ultra',
                'gemini-pro': 'Gemini Pro'
            };
            return nameMap[modelId] || modelId;
        },

        // Methods
        openProviderConfig(providerId) {
            const provider = this.providersList.find(p => p.id === providerId);
            if (provider) {
                this.currentProvider = provider;
                this.configOpen = true;
                this.validationMessage = '';
                this.validationIcon = '';
                // Reinitialize Lucide icons after modal opens
                this.$nextTick(() => {
                    if (typeof lucide !== 'undefined') {
                        lucide.createIcons();
                    }
                });
            }
        },

        openAddCustomProvider() {
            this.customProvider = {
                name: '',
                apiType: 'openai',
                baseUrl: '',
                modelsInput: '',
                apiKey: '',
                // Test connection states
                isTestingConnection: false,
                connectionStatus: '', // 'success', 'error', ''
                connectionMessage: '',
                // Fetch models states
                isFetchingModels: false,
                fetchStatus: '', // 'success', 'error', ''
                fetchMessage: '',
                // Error message
                fetchError: ''
            };
            this.customProviderOpen = true;
            // Reinitialize Lucide icons after modal opens
            this.$nextTick(() => {
                if (typeof lucide !== 'undefined') {
                    lucide.createIcons();
                }
            });
        },

        addCustomProvider() {
            if (!this.customProvider.name || !this.customProvider.apiType) return;

            const id = 'custom-' + this.customProvider.name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
            const models = this.customProvider.modelsInput
                ? this.customProvider.modelsInput.split(',').map(m => m.trim()).filter(m => m)
                : ['Default Model'];

            this.providersList.push({
                id: id,
                name: this.customProvider.name,
                icon: this.customProvider.name.substring(0, 2).toUpperCase(),
                iconClass: 'custom',
                models: models,
                configured: !!this.customProvider.apiKey,
                apiKey: this.customProvider.apiKey || '',
                isCustom: true,
                apiType: this.customProvider.apiType,
                baseUrl: this.customProvider.baseUrl || null
            });

            this.customProviderOpen = false;
        },

        testConnection() {
            if (!this.customProvider.apiKey || this.customProvider.apiKey.trim().length === 0) {
                this.customProvider.connectionStatus = 'error';
                this.customProvider.connectionMessage = 'Please enter an API key first';
                return;
            }

            this.customProvider.isTestingConnection = true;
            this.customProvider.connectionStatus = '';
            this.customProvider.connectionMessage = '';
            this.customProvider.fetchError = '';

            // Simulate connection test
            setTimeout(() => {
                this.customProvider.isTestingConnection = false;
                // For demo, consider any key with 5+ characters as valid
                if (this.customProvider.apiKey.length >= 5) {
                    this.customProvider.connectionStatus = 'success';
                    this.customProvider.connectionMessage = 'Connected';
                    this.customProvider.fetchError = '';
                } else {
                    this.customProvider.connectionStatus = 'error';
                    this.customProvider.connectionMessage = 'Connection failed';
                    this.customProvider.fetchError = 'Connection failed. Please check your API key and endpoint.';
                }
            }, 1000);
        },

        fetchModels() {
            if (!this.customProvider.apiKey || this.customProvider.apiKey.trim().length === 0) {
                this.customProvider.fetchStatus = 'error';
                this.customProvider.fetchMessage = 'Please enter an API key first';
                return;
            }

            // Require connection test first
            if (this.customProvider.connectionStatus !== 'success') {
                this.customProvider.fetchStatus = 'error';
                this.customProvider.fetchMessage = 'Please test connection first';
                return;
            }

            this.customProvider.isFetchingModels = true;
            this.customProvider.fetchStatus = '';
            this.customProvider.fetchMessage = '';
            this.customProvider.fetchError = '';

            // Simulate fetching models
            setTimeout(() => {
                this.customProvider.isFetchingModels = false;

                // Demo: return different models based on API type
                if (this.customProvider.apiType === 'openai') {
                    this.customProvider.modelsInput = 'GPT-4 Turbo, GPT-4, GPT-3.5 Turbo';
                } else if (this.customProvider.apiType === 'anthropic') {
                    this.customProvider.modelsInput = 'Claude Opus, Claude Sonnet, Claude Haiku';
                }

                this.customProvider.fetchStatus = 'success';
                this.customProvider.fetchMessage = 'Models fetched';
            }, 1500);
        },

        removeProvider(providerId) {
            const index = this.providersList.findIndex(p => p.id === providerId);
            if (index > -1) {
                // Clear any selected models from this provider
                Object.keys(this.selectedModels).forEach(op => {
                    const modelId = this.selectedModels[op];
                    if (modelId && modelId.startsWith(providerId)) {
                        this.selectedModels[op] = null;
                    }
                });
                this.providersList.splice(index, 1);
            }
        },

        validateApiKey() {
            if (!this.currentProvider.apiKey || this.currentProvider.apiKey.trim().length === 0) {
                this.validationIcon = 'alert-circle';
                this.validationMessage = 'Please enter an API key';
                this.validationClass = 'error';
                return;
            }

            this.validationIcon = 'loader-2';
            this.validationMessage = 'Validating...';
            this.validationClass = 'info';

            // Simulate validation
            setTimeout(() => {
                if (this.currentProvider.apiKey.length < 10) {
                    this.validationIcon = 'alert-circle';
                    this.validationMessage = 'Invalid API key. Please check and try again.';
                    this.validationClass = 'error';
                } else {
                    this.validationIcon = 'check-circle';
                    this.validationMessage = 'API key validated successfully!';
                    this.validationClass = 'success';
                }
            }, 1000);
        },

        saveProvider() {
            if (!this.currentProvider.apiKey || this.currentProvider.apiKey.trim().length === 0) return;

            this.currentProvider.configured = true;
            this.configOpen = false;
        },

        selectModel(operationType, modelId) {
            this.selectedModels[operationType] = modelId;
        },

        getAvailableModels(operationType) {
            const capabilities = {
                analysis: ['Claude Haiku', 'GPT-4', 'Gemini Pro'],
                code: ['Claude Opus', 'Claude Sonnet', 'GPT-4 Turbo', 'Gemini Ultra'],
                vision: ['Claude Opus', 'Claude Sonnet', 'GPT-4 Turbo', 'Gemini Ultra', 'Gemini Pro'],
                audio: ['Claude Opus', 'GPT-4 Turbo', 'Gemini Ultra'],
                tips: ['Claude Haiku', 'GPT-4', 'Gemini Pro'],
                review: ['Claude Sonnet', 'Claude Opus', 'GPT-4 Turbo', 'Gemini Ultra']
            };

            const models = [];
            this.providersList.forEach(provider => {
                if (!provider.configured) return;
                const allowed = capabilities[operationType] || [];
                provider.models.forEach(modelName => {
                    // For custom providers, include all models regardless of capability map
                    const isAllowed = provider.isCustom || allowed.includes(modelName);
                    if (!isAllowed) return;

                    const modelId = `${provider.id}-${modelName.toLowerCase().replace(/\s+/g, '-')}`;
                    models.push({
                        id: modelId,
                        name: modelName,
                        providerName: provider.name,
                        providerIcon: provider.icon,
                        providerIconClass: provider.iconClass
                    });
                });
            });
            return models;
        },

        getModelShortName(modelId) {
            const [, ...modelParts] = modelId.split('-');
            const modelName = modelParts.map(p => p.charAt(0).toUpperCase() + p.slice(1)).join(' ');
            const shortMap = {
                'Claude Opus': 'Opus',
                'Claude Sonnet': 'Sonnet',
                'Claude Haiku': 'Haiku',
                'GPT-4 Turbo': 'GPT-4T',
                'GPT-4': 'GPT-4',
                'Gemini Ultra': 'GemU',
                'Gemini Pro': 'GemP'
            };
            return shortMap[modelName] || modelName;
        },

        // Alpine.js init hook - called automatically when component initializes
        init() {
            // Initialize flow index watcher
            this.$watch('currentFlow', (flow) => {
                const flows = ['welcome', 'providers', 'settings', 'done'];
                this.flowIndex = flows.indexOf(flow);
            });

            // Initialize Lucide icons after DOM updates
            this.$nextTick(() => {
                if (typeof lucide !== 'undefined') {
                    lucide.createIcons();
                }
            });
        }
    };
}