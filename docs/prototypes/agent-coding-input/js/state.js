/**
 * Agent Coding Input Prototype State Management
 *
 * Alpine.js state for the agent coding input component prototype.
 * Handles all interactions including mode selection, dropdowns, file attachments,
 * command palette, and the improve with AI feature.
 */

// Inline SVG icons - no external dependencies
const ICONS = {
	'layout-list': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"></line><line x1="8" y1="12" x2="21" y2="12"></line><line x1="8" y1="18" x2="21" y2="18"></line><line x1="3" y1="6" x2="3.01" y2="6"></line><line x1="3" y1="12" x2="3.01" y2="12"></line><line x1="3" y1="18" x2="3.01" y2="18"></line></svg>',
	'code': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>',
	'chevron-down': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>',
	'loader-2': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"></path></svg>',
	'check': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>',
	'git-branch': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>',
	'sparkles': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18"></path><path d="M5.6 5.6l12.8 12.8"></path><path d="M3 12h18"></path><path d="M5.6 18.4l12.8-12.8"></path></svg>',
	'file': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>',
	'x': '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>',
	'book': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path></svg>',
	'refresh': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path></svg>',
	'check-circle': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>',
	'message-square': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>',
	'send': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" y1="2" x2="11" y2="13"></line><polygon points="22 2 15 22 11 13 2 9 22 2"></polygon></svg>',
	'sun-moon': '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0z"></path></svg>',
	'trash': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"></path><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path></svg>',
	'git-commit': '<svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><line x1="12" y1="3" x2="12" y2="9"></line><line x1="12" y1="15" x2="12" y2="21"></line></svg>'
};

function prototypeState() {
	return {
		// Theme
		theme: 'dark',

		// Input mode: 'plan' or 'edit'
		mode: 'plan',

		// Input text
		inputText: '',

		// Provider dropdown state
		providerDropdownOpen: false,
		selectedProvider: 'anthropic',
		providers: [
			{ id: 'anthropic', name: 'Anthropic', icon: 'bot' },
			{ id: 'openai', name: 'OpenAI', icon: 'zap' },
			{ id: 'google', name: 'Google', icon: 'sparkles' },
			{ id: 'local', name: 'Local LLM', icon: 'cpu' }
		],

		// Model dropdown state
		modelDropdownOpen: false,
		selectedModel: 'claude-opus-4',
		models: {
			anthropic: [
				{ id: 'claude-opus-4', name: 'Claude Opus 4', tier: 'Premium' },
				{ id: 'claude-sonnet-4', name: 'Claude Sonnet 4', tier: 'Standard' },
				{ id: 'claude-haiku-4', name: 'Claude Haiku 4', tier: 'Fast' }
			],
			openai: [
				{ id: 'gpt-4-turbo', name: 'GPT-4 Turbo', tier: 'Premium' },
				{ id: 'gpt-4', name: 'GPT-4', tier: 'Standard' },
				{ id: 'gpt-3.5-turbo', name: 'GPT-3.5 Turbo', tier: 'Fast' }
			],
			google: [
				{ id: 'gemini-pro', name: 'Gemini Pro', tier: 'Standard' },
				{ id: 'gemini-flash', name: 'Gemini Flash', tier: 'Fast' }
			],
			local: [
				{ id: 'codellama-34b', name: 'Code Llama 34B', tier: 'Local' },
				{ id: 'mistral-7b', name: 'Mistral 7B', tier: 'Local' }
			]
		},

		// Branch state
		branchDropdownOpen: false,
		selectedBranch: 'main',
		branches: ['main', 'develop', 'feature/auth-flow', 'bugfix/input-validation'],

		// File attachments
		attachedFiles: [],

		// Command palette state
		commandPaletteOpen: false,
		commandPaletteType: null, // 'files' or 'commands'
		commandQuery: '',
		selectedCommandIndex: 0,
		commandPalettePosition: { top: '0px', left: '0px', width: '0px' },

		// Available files for @ mention
		availableFiles: [
			{ name: 'src-ui/src/lib/components/AgentInput.svelte', type: 'component' },
			{ name: 'src-ui/src/lib/types/command.ts', type: 'types' },
			{ name: 'src-ui/src/lib/styles/design-system.css', type: 'styles' },
			{ name: 'src/backend/src/agent/mod.rs', type: 'rust' },
			{ name: 'Cargo.toml', type: 'config' },
			{ name: 'README.md', type: 'docs' }
		],

		// Available commands for / mention
		availableCommands: [
			{ id: 'generate', name: 'generate', description: 'Generate code from description', icon: 'code', category: 'Code' },
			{ id: 'explain', name: 'explain', description: 'Explain selected code', icon: 'book', category: 'Code' },
			{ id: 'refactor', name: 'refactor', description: 'Refactor selected code', icon: 'refresh', category: 'Code' },
			{ id: 'test', name: 'test', description: 'Generate tests for code', icon: 'check-circle', category: 'Code' },
			{ id: 'new-chat', name: 'new-chat', description: 'Start a new conversation', icon: 'message-square', category: 'Conversation', shortcut: 'Ctrl+N' },
			{ id: 'clear', name: 'clear', description: 'Clear current conversation', icon: 'trash', category: 'Conversation' },
			{ id: 'goto-file', name: 'goto-file', description: 'Go to file', icon: 'file', category: 'Navigation', shortcut: 'Ctrl+P' },
			{ id: 'git-commit', name: 'git-commit', description: 'Create a commit', icon: 'git-commit', category: 'Git' }
		],

		// Improve with AI state
		improveLoading: false,
		improveSuggestion: '',

		// Send button state
		canSend: false,
		isSending: false,

		// Example configuration for different demo states
		exampleStates: {
			default: {
				inputText: '',
				attachedFiles: [],
				commandPaletteOpen: false
			},
			withFiles: {
				inputText: 'Help me refactor the authentication flow in ',
				attachedFiles: ['src-ui/src/lib/components/AgentInput.svelte', 'src-ui/src/lib/types/command.ts'],
				commandPaletteOpen: false
			},
			withCommandPalette: {
				inputText: '/',
				attachedFiles: [],
				commandPaletteOpen: true,
				commandPaletteType: 'commands'
			},
			withFilesAndPalette: {
				inputText: 'Can you explain how the ',
				attachedFiles: ['src-ui/src/lib/components/AgentInput.svelte'],
				commandPaletteOpen: true,
				commandPaletteType: 'files'
			}
		},

		// Current example state (for demo purposes)
		currentExample: 'default',

		/**
		 * Get available models for selected provider
		 */
		get availableModels() {
			return this.models[this.selectedProvider] || [];
		},

		/**
		 * Get filtered files based on command query
		 */
		get filteredFiles() {
			if (!this.commandQuery) {
				return this.availableFiles;
			}
			const query = this.commandQuery.toLowerCase();
			return this.availableFiles.filter(file =>
				file.name.toLowerCase().includes(query)
			);
		},

		/**
		 * Get filtered commands based on command query
		 */
		get filteredCommands() {
			if (!this.commandQuery) {
				return this.availableCommands;
			}
			const query = this.commandQuery.toLowerCase().replace('/', '');
			return this.availableCommands.filter(cmd =>
				cmd.name.toLowerCase().includes(query) ||
				cmd.description.toLowerCase().includes(query)
			);
		},

		/**
		 * Get grouped commands by category
		 */
		get groupedCommands() {
			const groups = {};
			for (const cmd of this.filteredCommands) {
				if (!groups[cmd.category]) {
					groups[cmd.category] = [];
				}
				groups[cmd.category].push(cmd);
			}
			return groups;
		},

		// Expose ICONS globally for x-html bindings
		ICONS: ICONS,

		/**
		 * Get command icon SVG by name
		 */
		getCommandIcon(iconName) {
			return ICONS[iconName] || ICONS['file'];
		},

		/**
		 * Initialize the prototype
		 */
		init() {
			// Make ICONS available globally for Alpine x-html bindings
			window.ICONS = ICONS;

			// Close dropdowns when clicking outside
			document.addEventListener('click', this.handleOutsideClick.bind(this));

			// Handle keyboard shortcuts
			document.addEventListener('keydown', this.handleGlobalKeydown.bind(this));

			// Update palette position on scroll/resize
			window.addEventListener('scroll', this.handleScroll.bind(this), true);
			window.addEventListener('resize', this.handleScroll.bind(this));
		},

		/**
		 * Handle scroll/resize events to update palette position
		 */
		handleScroll() {
			if (this.commandPaletteOpen && this.$refs.textarea) {
				this.updateCommandPalettePosition(this.$refs.textarea);
			}
		},

		/**
		 * Toggle between plan and edit mode
		 */
		toggleMode(newMode) {
			this.mode = newMode;
		},

		/**
		 * Toggle provider dropdown
		 */
		toggleProviderDropdown() {
			this.providerDropdownOpen = !this.providerDropdownOpen;
			this.modelDropdownOpen = false;
			this.branchDropdownOpen = false;
		},

		/**
		 * Select a provider
		 */
		selectProvider(providerId) {
			this.selectedProvider = providerId;
			// Select first model of new provider
			const models = this.models[providerId];
			if (models && models.length > 0) {
				this.selectedModel = models[0].id;
			}
			this.providerDropdownOpen = false;
		},

		/**
		 * Toggle model dropdown
		 */
		toggleModelDropdown() {
			this.modelDropdownOpen = !this.modelDropdownOpen;
			this.providerDropdownOpen = false;
			this.branchDropdownOpen = false;
		},

		/**
		 * Select a model
		 */
		selectModel(modelId) {
			this.selectedModel = modelId;
			this.modelDropdownOpen = false;
		},

		/**
		 * Toggle branch dropdown
		 */
		toggleBranchDropdown() {
			this.branchDropdownOpen = !this.branchDropdownOpen;
			this.providerDropdownOpen = false;
			this.modelDropdownOpen = false;
		},

		/**
		 * Select a branch
		 */
		selectBranch(branch) {
			this.selectedBranch = branch;
			this.branchDropdownOpen = false;
		},

		/**
		 * Handle input changes
		 */
		handleInput(event) {
			const value = event.target.value;
			this.inputText = value;
			this.canSend = value.trim().length > 0;

			// Check if we should show command palette
			const cursorPosition = event.target.selectionStart;
			const textBeforeCursor = value.substring(0, cursorPosition);

			// Check for @ trigger (file mention)
			const atMatch = textBeforeCursor.match(/@(\w*)$/);
			if (atMatch) {
				this.commandPaletteOpen = true;
				this.commandPaletteType = 'files';
				this.commandQuery = atMatch[1];
				this.selectedCommandIndex = 0;
				this.updateCommandPalettePosition(event.target);
				return;
			}

			// Check for / trigger (command palette)
			const slashMatch = textBeforeCursor.match(/\/(\w*)$/);
			if (slashMatch) {
				this.commandPaletteOpen = true;
				this.commandPaletteType = 'commands';
				this.commandQuery = slashMatch[1];
				this.selectedCommandIndex = 0;
				this.updateCommandPalettePosition(event.target);
				return;
			}

			// Close command palette if no trigger
			if (this.commandPaletteOpen) {
				this.commandPaletteOpen = false;
				this.commandQuery = '';
			}
		},

		/**
		 * Update command palette position based on cursor
		 */
		updateCommandPalettePosition(textarea) {
			this.$nextTick(() => {
				const rect = textarea.getBoundingClientRect();
				const scrollPadding = 8; // Extra padding from scroll position

				// Position below the textarea, matching its width and left position
				// This ensures the palette is not restricted by the input area
				this.commandPalettePosition = {
					top: `${rect.bottom + scrollPadding}px`,
					left: `${rect.left}px`,
					width: `${rect.width}px`
				};
			});
		},

		/**
		 * Handle keyboard navigation in command palette
		 */
		handleKeydown(event) {
			if (!this.commandPaletteOpen) return;

			const items = this.commandPaletteType === 'files'
				? this.filteredFiles
				: this.filteredCommands;

			if (event.key === 'ArrowDown') {
				event.preventDefault();
				this.selectedCommandIndex = Math.min(this.selectedCommandIndex + 1, items.length - 1);
			} else if (event.key === 'ArrowUp') {
				event.preventDefault();
				this.selectedCommandIndex = Math.max(this.selectedCommandIndex - 1, 0);
			} else if (event.key === 'Enter') {
				event.preventDefault();
				const selectedItem = items[this.selectedCommandIndex];
				if (selectedItem) {
					this.selectCommandItem(selectedItem);
				}
			} else if (event.key === 'Escape') {
				event.preventDefault();
				this.commandPaletteOpen = false;
				this.commandQuery = '';
			}
		},

		/**
		 * Select an item from the command palette
		 */
		selectCommandItem(item) {
			const cursorPosition = this.$refs.textarea?.selectionStart || this.inputText.length;
			const textBeforeCursor = this.inputText.substring(0, cursorPosition);
			const textAfterCursor = this.inputText.substring(cursorPosition);

			if (this.commandPaletteType === 'files') {
				// Remove the @ trigger and add file reference
				const beforeMention = textBeforeCursor.replace(/@\w*$/, '');
				this.inputText = beforeMention + '@' + item.name + ' ' + textAfterCursor;
				this.attachedFiles.push(item.name);
			} else {
				// Remove the / trigger and add command
				const beforeCommand = textBeforeCursor.replace(/\/\w*$/, '');
				this.inputText = beforeCommand + '/' + item.name + ' ' + textAfterCursor;
			}

			this.commandPaletteOpen = false;
			this.commandQuery = '';
			this.canSend = true;

			// Focus textarea after selection
			this.$nextTick(() => {
				this.$refs.textarea?.focus();
			});
		},

		/**
		 * Remove an attached file
		 */
		removeFile(fileName) {
			const index = this.attachedFiles.indexOf(fileName);
			if (index > -1) {
				this.attachedFiles.splice(index, 1);
				// Also remove from input text
				this.inputText = this.inputText.replace('@' + fileName, '');
			}
		},

		/**
		 * Improve the input text with AI
		 */
		async improveWithAI() {
			if (!this.inputText.trim() || this.improveLoading) return;

			this.improveLoading = true;

			// Simulate AI improvement
			await new Promise(resolve => setTimeout(resolve, 1500));

			// Demo improvement - in real implementation, this would call an AI API
			const improvements = [
				'Create a comprehensive authentication system with JWT tokens, refresh token rotation, and secure password hashing using bcrypt.',
				'Refactor the authentication flow by implementing a centralized auth service with proper error handling and token management.',
				'Add complete unit tests for the authentication module, covering login, logout, token refresh, and password reset scenarios.'
			];

			this.improveSuggestion = improvements[Math.floor(Math.random() * improvements.length)];
			this.inputText = this.improveSuggestion;
			this.canSend = true;
			this.improveLoading = false;

			// Clear suggestion after a delay
			setTimeout(() => {
				this.improveSuggestion = '';
			}, 3000);
		},

		/**
		 * Send the instruction
		 */
		sendInstruction() {
			if (!this.canSend && !this.isSending) return;

			// If currently sending, cancel/stop
			if (this.isSending) {
				this.isSending = false;
				console.log('Sending cancelled/stopped by user');
				return;
			}

			// Start sending
			this.isSending = true;

			// In a real implementation, this would send the instruction to the agent
			console.log('Sending instruction:', {
				mode: this.mode,
				provider: this.selectedProvider,
				model: this.selectedModel,
				branch: this.selectedBranch,
				text: this.inputText,
				files: this.attachedFiles
			});

			// Simulate async sending - in real implementation this would be an actual async call
			// For demo, we'll keep it in running state until user clicks stop
		},

		/**
		 * Handle clicks outside dropdowns
		 */
		handleOutsideClick(event) {
			// Close dropdowns if clicking outside
			if (!event.target.closest('.dropdown')) {
				this.providerDropdownOpen = false;
				this.modelDropdownOpen = false;
				this.branchDropdownOpen = false;
			}

			// Close command palette if clicking outside
			if (!event.target.closest('.command-palette') &&
				!event.target.closest('.input-textarea')) {
				this.commandPaletteOpen = false;
			}
		},

		/**
		 * Handle global keyboard shortcuts
		 */
		handleGlobalKeydown(event) {
			// Ctrl/Cmd + K to open command palette
			if ((event.ctrlKey || event.metaKey) && event.key === 'k') {
				event.preventDefault();
				this.commandPaletteOpen = true;
				this.commandPaletteType = 'commands';
				this.commandQuery = '';
			}

			// Ctrl/Cmd + Enter to send
			if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
				event.preventDefault();
				this.sendInstruction();
			}
		},

		/**
		 * Set example state for demo purposes
		 */
		setExample(exampleKey) {
			const example = this.exampleStates[exampleKey];
			if (example) {
				this.currentExample = exampleKey;
				this.inputText = example.inputText;
				this.attachedFiles = [...example.attachedFiles];
				this.commandPaletteOpen = example.commandPaletteOpen;
				this.commandPaletteType = example.commandPaletteType;
				this.commandQuery = example.inputText.substring(example.inputText.lastIndexOf('@') + 1 || example.inputText.lastIndexOf('/') + 1);
				this.canSend = example.inputText.trim().length > 0;
			}
		},

		/**
		 * Toggle theme
		 */
		toggleTheme() {
			this.theme = this.theme === 'dark' ? 'light' : 'dark';
			document.documentElement.setAttribute('data-theme', this.theme);
		}
	};
}
