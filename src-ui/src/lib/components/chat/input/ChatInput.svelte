<script lang="ts">
	/**
	 * ChatInput Component
	 * Enhanced multi-line text input with command palette, mentions, and intent detection.
	 * Supports both frontend-only mode (simple patterns) and backend NLP mode.
	 */
	import styles from './ChatInput.module.css';
	import CommandPalette from './CommandPalette.svelte';
	import IntentIndicator from './IntentIndicator.svelte';
	import MentionMenu from './MentionMenu.svelte';
	import ModelSelector from './ModelSelector.svelte';
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import { ArrowUpIcon, CloseIcon, CodeIcon, FileIcon, PaperclipIcon, ZapIcon } from '$lib/icons';
	import type { Command } from '$lib/types/command';
	import { BUILT_IN_COMMANDS, filterCommands } from '$lib/types/command';
	import type { IntentClassification } from '$lib/types/nlp';
	import { agentStore, chatStore, isLlmStreaming, nlpState } from '$lib/stores';
	import { get } from 'svelte/store';
	import { isValidModelId, sanitizeModelId } from '$lib/utils/validation';
	import { createLogger } from '$lib/utils/logger';

	const log = createLogger('ChatInput');

	interface FileAttachment {
		id: string;
		name: string;
		type: string;
		size: number;
		preview?: string;
	}

	interface MentionItem {
		id: string;
		name: string;
		type: 'file' | 'agent' | 'context';
		icon?: string;
	}

	interface Props {
		/** Placeholder text */
		placeholder?: string;
		/** Current input value */
		value: string;
		/** Value change handler */
		onchange: (value: string) => void;
		/** Submit handler */
		onsubmit: () => void;
		/** Focus handler */
		onfocus?: () => void;
		/** Blur handler */
		onblur?: () => void;
		/** Disabled state */
		disabled?: boolean;
		/** Loading state */
		loading?: boolean;
		/** Whether the thread-linked agent session is currently active */
		isAgentWorking?: boolean;
		/** Session ID linked to the visible thread */
		agentSessionId?: string | null;
		/** Attached files */
		attachments?: FileAttachment[];
		/** File attach handler */
		onattach?: (files: File[]) => void;
		/** Remove attachment handler */
		onremoveattachment?: (id: string) => void;
		/** Maximum character limit */
		maxLength?: number;
		/** Available commands for palette */
		commands?: Command[];
		/** Available mention items */
		mentionItems?: MentionItem[];
		/** Show intent indicator */
		showIntent?: boolean;
		/** Enable markdown preview */
		enableMarkdownPreview?: boolean;
		/** Use backend NLP for intent detection (default: true when available) */
		useBackendNlp?: boolean;
		/** Selected model ID */
		modelId?: string | null;
		/** Model change handler */
		onModelChange?: (modelId: string) => void;
		/** Open settings handler */
		onOpenSettings?: () => void;
	}

	let {
		placeholder = 'Type a message or command...',
		value,
		onchange,
		onsubmit,
		onfocus,
		onblur,
		disabled = false,
		loading = false,
		isAgentWorking = false,
		agentSessionId = null,
		attachments = [],
		onattach,
		onremoveattachment,
		maxLength,
		commands = BUILT_IN_COMMANDS,
		mentionItems = [],
		showIntent = true,
		enableMarkdownPreview = false,
		useBackendNlp = true,
		modelId = null,
		onModelChange,
		onOpenSettings
	}: Props = $props();

	let textareaRef: HTMLTextAreaElement | undefined = $state();
	let fileInputRef: HTMLInputElement | undefined = $state();

	// UI state
	let showCommandPalette = $state(false);
	let showMentionMenu = $state(false);
	let commandQuery = $state('');
	let mentionQuery = $state('');
	let isPreviewMode = $state(false);
	let cursorPosition = $state(0);
	let isShaking = $state(false);
	let isSending = $state(false);

// Get YOLO mode from the selected thread
	let yoloMode = $derived(
		$chatStore.selectedThreadId
			? $chatStore.threads.find((t) => t.id === $chatStore.selectedThreadId)?.metadata
					.yoloMode ?? false
			: false
	);

	// Note: $nlpState and $isLlmStreaming auto-subscribe to stores from $lib/stores

	// Derived values
	let filteredCommands = $derived(filterCommands(commands, commandQuery));
	let filteredMentions = $derived(
		mentionItems.filter((item) => item.name.toLowerCase().includes(mentionQuery.toLowerCase()))
	);

	// Determine if the agent is actively working (LLM streaming OR Agent running)
	let isWorking = $derived(isAgentWorking || $isLlmStreaming || loading || isSending);

	// Dynamic placeholder based on context
	let dynamicPlaceholder = $derived(() => {
		if (isWorking) return 'Working...';
		if (isSending) return '';
		if (value.startsWith('/')) return 'Select a command...';
		if (value.startsWith('@')) return 'Reference files, agents...';
		return placeholder;
	});

	// Intent detection: use backend if available and enabled, otherwise fallback to frontend
	let detectedIntent = $derived(() => {
		if (useBackendNlp && $nlpState.backendAvailable && $nlpState.detectedIntent) {
			return $nlpState.detectedIntent;
		}
		// Fallback to frontend detection
		return detectIntentFrontend(value);
	});

	let intentLoading = $derived(useBackendNlp && $nlpState.processing);
	let characterCount = $derived(value.length);
	let isOverLimit = $derived(maxLength !== undefined && characterCount > maxLength);
	let canSubmit = $derived(!disabled && !loading && value.trim() && !isOverLimit);

	let containerClasses = $derived(
		[
			styles.container,
			disabled && styles.disabled,
			isPreviewMode && styles.previewMode,
			isShaking && styles.shake,
			isWorking && styles.isWorking
		]
			.filter(Boolean)
			.join(' ')
	);

	// Validate and sanitize model ID
	let validatedModelId = $derived(sanitizeModelId(modelId));

	// Log warning for invalid model IDs (development aid)
	$effect(() => {
		if (modelId !== null && !isValidModelId(modelId)) {
			log.warn('Invalid modelId prop received:', modelId);
		}
	});

	// Auto-resize textarea (max 120px height)
	$effect(() => {
		if (textareaRef && !isPreviewMode) {
			textareaRef.style.height = 'auto';
			textareaRef.style.height = `${Math.min(textareaRef.scrollHeight, 120)}px`;
		}
	});

	// Trigger backend intent classification when value changes
	$effect(() => {
		if (useBackendNlp && value.trim().length > 3) {
			chatStore.classifyInput(value);
		}
	});

	// Reset isSending when value is cleared (successful send)
	$effect(() => {
		if (!value) {
			isSending = false;
		}
	});

	async function handleSendMessage() {
		// Handle Stop action if agent is working
		if (isWorking) {
			if ($isLlmStreaming) {
				await chatStore.cancelLlmStream();
			} else if (agentSessionId) {
				await agentStore.cancelSession(agentSessionId);
			} else {
				const state = get(agentStore);
				if (state.activeSessionId) {
					await agentStore.cancelSession(state.activeSessionId);
				}
			}
			isSending = false;
			return;
		}

		if (!value.trim() || isOverLimit) {
			isShaking = true;
			setTimeout(() => {
				isShaking = false;
			}, 400); // Duration of shake animation
			return;
		}

		if (canSubmit) {
			isSending = true;
			// Small delay for the "fly out" animation
			await new Promise((resolve) => setTimeout(resolve, 150));
			onsubmit();

			// Safety check: if loading hasn't started after a short delay and value persists,
			// reset isSending so user isn't staring at invisible text (e.g. error or ignored submit)
			setTimeout(() => {
				if (!loading && value) {
					isSending = false;
				}
			}, 200);
		}
	}

	/**
	 * Frontend-only intent detection (fallback when backend is unavailable).
	 */
	function detectIntentFrontend(text: string): IntentClassification | null {
		const lowerText = text.toLowerCase().trim();

		if (!lowerText) return null;

		// Simple intent detection patterns
		const patterns: { pattern: RegExp; intent: string }[] = [
			{ pattern: /^(generate|create|write|build|make)\s/i, intent: 'generate_code' },
			{ pattern: /^(explain|what|why|how|describe)\s/i, intent: 'explain_code' },
			{ pattern: /^(fix|debug|solve|resolve|repair)\s/i, intent: 'find_bugs' },
			{ pattern: /^(refactor|improve|optimize|clean)\s/i, intent: 'refactor_code' },
			{ pattern: /^(test|spec|verify)\s/i, intent: 'generate_test' },
			{ pattern: /^(review|check|analyze)\s/i, intent: 'analyze_code' },
			{ pattern: /^(document|doc|comment)\s/i, intent: 'document_code' }
		];

		for (const { pattern, intent } of patterns) {
			if (pattern.test(lowerText)) {
				return {
					intent: intent as any,
					confidence: 0.7,
					alternative_intents: []
				};
			}
		}

		return null;
	}

	function handleInput(e: Event) {
		const target = e.target as HTMLTextAreaElement;
		onchange(target.value);
		checkForTriggers(target.value, target.selectionStart);
	}

	function checkForTriggers(text: string, position: number) {
		cursorPosition = position;

		// Check for command trigger (/)
		const beforeCursor = text.slice(0, position);
		const lastSlashIndex = beforeCursor.lastIndexOf('/');

		if (lastSlashIndex !== -1 && !text.slice(lastSlashIndex, position).includes(' ')) {
			commandQuery = text.slice(lastSlashIndex + 1, position);
			showCommandPalette = true;
			showMentionMenu = false;
			return;
		}

		// Check for mention trigger (@)
		const lastAtIndex = beforeCursor.lastIndexOf('@');

		if (lastAtIndex !== -1 && !text.slice(lastAtIndex, position).includes(' ')) {
			mentionQuery = text.slice(lastAtIndex + 1, position);
			showMentionMenu = true;
			showCommandPalette = false;
			return;
		}

		showCommandPalette = false;
		showMentionMenu = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		// Handle command palette navigation
		if (showCommandPalette || showMentionMenu) {
			if (e.key === 'Escape') {
				showCommandPalette = false;
				showMentionMenu = false;
				e.preventDefault();
				return;
			}
			if (e.key === 'Enter' && !e.shiftKey) {
				// Let the palette/menu handle Enter
				return;
			}
		}

		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSendMessage();
		}
	}

	function handleCommandSelect(command: Command) {
		const beforeCursor = value.slice(0, cursorPosition);
		const afterCursor = value.slice(cursorPosition);
		const lastSlashIndex = beforeCursor.lastIndexOf('/');

		const newValue = beforeCursor.slice(0, lastSlashIndex) + `/${command.name} ` + afterCursor;

		onchange(newValue);
		showCommandPalette = false;
		textareaRef?.focus();
	}

	function handleMentionSelect(mention: MentionItem) {
		const beforeCursor = value.slice(0, cursorPosition);
		const afterCursor = value.slice(cursorPosition);
		const lastAtIndex = beforeCursor.lastIndexOf('@');

		const newValue = beforeCursor.slice(0, lastAtIndex) + `@${mention.name} ` + afterCursor;

		onchange(newValue);
		showMentionMenu = false;
		textareaRef?.focus();
	}

	function handleFileSelect(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files && onattach) {
			onattach(Array.from(target.files));
			target.value = '';
		}
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function togglePreview() {
		isPreviewMode = !isPreviewMode;
	}
</script>

<div class={containerClasses}>
	<!-- Attachments -->
	{#if attachments.length > 0}
		<div class={styles.attachments}>
			{#each attachments as file (file.id)}
				<div class={styles.attachment}>
					{#if file.preview}
						<img src={file.preview} alt={file.name} class={styles.attachmentPreview} />
					{:else}
						<FileIcon class={styles.fileIcon} size={16} />
					{/if}
					<span class={styles.attachmentName}>{file.name}</span>
					<span class={styles.attachmentSize}>{formatSize(file.size)}</span>
					<IconButton
						label="Remove {file.name}"
						tooltip="Remove"
						variant="ghost"
						intent="danger"
						size="xs"
						onclick={() => onremoveattachment?.(file.id)}
					>
						{#snippet icon()}
							<CloseIcon size={14} />
						{/snippet}
					</IconButton>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Intent indicator -->
	{#if showIntent && detectedIntent()}
		<div class={styles.intentWrapper}>
			<IntentIndicator
				intent={detectedIntent()!.intent}
				confidence={detectedIntent()!.confidence}
				showConfidence={true}
				loading={intentLoading}
				classification={detectedIntent()}
			/>
		</div>
	{/if}

	<div class={styles.inputWrapper}>
		<textarea
			bind:this={textareaRef}
			class={[styles.textarea, isSending && styles.sending].filter(Boolean).join(' ')}
			placeholder={dynamicPlaceholder()}
			{value}
			disabled={disabled || isWorking}
			oninput={handleInput}
			onkeydown={handleKeydown}
			{onfocus}
			{onblur}
			rows={1}
			aria-label="Message input"
			aria-describedby={showCommandPalette || showMentionMenu ? 'input-menu' : undefined}
		></textarea>

		<!-- Command palette -->
		{#if showCommandPalette}
			<CommandPalette
				query={commandQuery}
				commands={filteredCommands}
				onSelect={handleCommandSelect}
				onClose={() => (showCommandPalette = false)}
			/>
		{/if}

		<!-- Mention menu -->
		{#if showMentionMenu}
			<MentionMenu
				query={mentionQuery}
				items={filteredMentions}
				onSelect={handleMentionSelect}
				onClose={() => (showMentionMenu = false)}
			/>
		{/if}
	</div>

	<!-- Actions bar -->
	<div class={styles.actions}>
		<div class={styles.leftActions}>
			{#if validatedModelId !== null && onModelChange}
				<ModelSelector selectedModelId={validatedModelId} {onModelChange} {onOpenSettings} />
			{/if}
			{#if $chatStore.selectedThreadId}
				<IconButton
					label="YOLO Mode"
					tooltip={yoloMode ? 'Disable YOLO Mode' : 'Enable YOLO Mode'}
					variant="ghost"
					size="sm"
					disabled={disabled}
					yoloActive={yoloMode}
					onclick={async () => {
						const selectedThreadId = $chatStore.selectedThreadId;
						if (selectedThreadId) {
							await chatStore.setYoloMode(selectedThreadId, !yoloMode);
						}
					}}
				>
					{#snippet icon()}
						<ZapIcon size={18} />
					{/snippet}
				</IconButton>
			{/if}
			{#if onattach}
				<IconButton
					label="Attach file"
					tooltip="Attach file"
					variant="ghost"
					size="sm"
					disabled={disabled || isWorking}
					onclick={() => fileInputRef?.click()}
				>
					{#snippet icon()}
						<PaperclipIcon size={18} />
					{/snippet}
				</IconButton>
				<input
					bind:this={fileInputRef}
					type="file"
					multiple
					accept="image/*,.pdf,.txt,.md"
					onchange={handleFileSelect}
					class={styles.fileInput}
				/>
			{/if}

			{#if enableMarkdownPreview}
				<IconButton
					label={isPreviewMode ? 'Edit mode' : 'Preview markdown'}
					tooltip={isPreviewMode ? 'Edit mode' : 'Preview markdown'}
					variant="ghost"
					size="sm"
					disabled={disabled || isWorking}
					active={isPreviewMode}
					onclick={togglePreview}
				>
					{#snippet icon()}
						<CodeIcon size={18} />
					{/snippet}
				</IconButton>
			{/if}
		</div>

		<div class={styles.rightActions}>
			{#if isWorking}
				<span class={styles.statusText}>
					{#if $isLlmStreaming}
						Generating response
					{:else if agentSessionId}
						Executing tasks
					{:else}
						Processing
					{/if}
				</span>
			{/if}

			{#if maxLength && !isWorking}
				<span
					class={[styles.characterCount, isOverLimit && styles.overLimit].filter(Boolean).join(' ')}
				>
					{characterCount}/{maxLength}
				</span>
			{/if}

			<button
				class={[
					isWorking ? styles.stopButton : styles.sendButton,
					!isWorking && canSubmit && styles.canSubmit
				]
					.filter(Boolean)
					.join(' ')}
				disabled={!isWorking && !canSubmit}
				onclick={handleSendMessage}
				aria-label={isWorking ? 'Stop generation' : 'Send message'}
				title={isWorking ? 'Stop' : 'Send'}
				type="button"
			>
				{#if isWorking}
					<div class={styles.stopIcon}></div>
				{:else}
					<ArrowUpIcon size={16} strokeWidth={3} />
				{/if}
			</button>
		</div>
	</div>
</div>
