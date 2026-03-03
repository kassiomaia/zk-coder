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
	import IconButton from '$lib/components/shared/IconButton.svelte';
	import type { Command } from '$lib/types/command';
	import { BUILT_IN_COMMANDS, filterCommands } from '$lib/types/command';
	import type { IntentClassification } from '$lib/types/nlp';
	import { chatStore, nlpState } from '$lib/stores';
	import { isBackendAvailable } from '$lib/api/nlp';

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
		/** Disabled state */
		disabled?: boolean;
		/** Loading state */
		loading?: boolean;
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
	}

	let {
		placeholder = '',
		value,
		onchange,
		onsubmit,
		disabled = false,
		loading = false,
		attachments = [],
		onattach,
		onremoveattachment,
		maxLength,
		commands = BUILT_IN_COMMANDS,
		mentionItems = [],
		showIntent = true,
		enableMarkdownPreview = false,
		useBackendNlp = true
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

	// Note: $nlpState auto-subscribes to nlpState store from $lib/stores

	// Derived values
	let filteredCommands = $derived(filterCommands(commands, commandQuery));
	let filteredMentions = $derived(
		mentionItems.filter((item) => item.name.toLowerCase().includes(mentionQuery.toLowerCase()))
	);

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
			isShaking && styles.shake
		]
			.filter(Boolean)
			.join(' ')
	);

	// Auto-resize textarea (max 3 lines = ~72px)
	$effect(() => {
		if (textareaRef && !isPreviewMode) {
			textareaRef.style.height = 'auto';
			textareaRef.style.height = `${Math.min(textareaRef.scrollHeight, 72)}px`;
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
						<svg
							class={styles.fileIcon}
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
							<polyline points="14 2 14 8 20 8" />
						</svg>
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
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<line x1="18" y1="6" x2="6" y2="18" />
								<line x1="6" y1="6" x2="18" y2="18" />
							</svg>
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

	<!-- Input area with palette/menu -->
	<div class={styles.inputWrapper}>
		<textarea
			bind:this={textareaRef}
			class={[styles.textarea, isSending && styles.sending].filter(Boolean).join(' ')}
			{placeholder}
			{value}
			{disabled}
			oninput={handleInput}
			onkeydown={handleKeydown}
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
			{#if onattach}
				<IconButton
					label="Attach file"
					tooltip="Attach file"
					variant="ghost"
					size="sm"
					disabled={disabled}
					onclick={() => fileInputRef?.click()}
				>
					{#snippet icon()}
						<svg
							width="20"
							height="20"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path
								d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48"
							/>
						</svg>
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
					disabled={disabled}
					active={isPreviewMode}
					onclick={togglePreview}
				>
					{#snippet icon()}
						<svg
							width="20"
							height="20"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
							stroke-linecap="round"
							stroke-linejoin="round"
						>
							<path d="M4 7V4h16v3" />
							<path d="M9 20h6" />
							<path d="M12 4v16" />
						</svg>
					{/snippet}
				</IconButton>
			{/if}
		</div>

		<div class={styles.rightActions}>
			{#if maxLength}
				<span
					class={[styles.characterCount, isOverLimit && styles.overLimit].filter(Boolean).join(' ')}
				>
					{characterCount}/{maxLength}
				</span>
			{/if}

			<IconButton
				label="Send message"
				tooltip="Send"
				variant="subtle"
				intent={canSubmit ? 'success' : 'default'}
				size="sm"
				disabled={!canSubmit}
				loading={loading || isSending}
				onclick={handleSendMessage}
				class={styles.sendButton}
			>
				{#snippet icon()}
					{#if loading}
						<svg
							class={styles.spinner}
							width="16"
							height="16"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<circle cx="12" cy="12" r="10" stroke-dasharray="60" stroke-dashoffset="20">
								<animateTransform
									attributeName="transform"
									type="rotate"
									from="0 12 12"
									to="360 12 12"
									dur="1s"
									repeatCount="indefinite"
								/>
							</circle>
						</svg>
					{:else}
						<svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
							<path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
						</svg>
					{/if}
				{/snippet}
			</IconButton>
		</div>
	</div>
</div>
