<script lang="ts">
	/**
	 * AgentClarificationPrompt
	 *
	 * Surfaces the pending clarification question emitted by
	 * `agent_narrate` with `intent = "question"` (Workstream E.2/E.4).
	 * The harness is paused until the user replies or the clarification
	 * timeout fires on the backend. Reply submission re-uses the
	 * `sendRequest` path in `agentStore`, the same IPC the main chat
	 * input uses — no duplicate send logic.
	 *
	 * Visual identity is attribution-first: the component reads as the
	 * agent asking the user a question, not as a warning banner. It
	 * mirrors the ChatMessage sender/content grammar so the reader's
	 * eye recognises it as an agent turn.
	 *
	 * Accessibility:
	 * - `<section>` labelled by the prompt so screen readers announce
	 *   the question before the composer controls.
	 * - Textarea has an explicit `<label>` (visually hidden).
	 * - Enter submits, Shift+Enter inserts a newline.
	 * - Optional structured `suggestions` render as `<button type="button">`s
	 *   that submit the suggestion's `label` through the same reply path.
	 */
	import { MessageCircleQuestion, Send } from 'lucide-svelte';
	import type { AwaitingClarificationDetails } from '$lib/types/agent';

	/**
	 * Optional structured suggestion. When the backend provides one or
	 * more suggestions, they render as quick-reply buttons above the
	 * composer. The `label` is submitted verbatim as the reply content.
	 * Today no backend path produces suggestions — the prop is future-ready.
	 */
	export interface ClarificationSuggestion {
		label: string;
		variant?: 'primary' | 'secondary';
	}

	interface Props {
		sessionId: string;
		details: AwaitingClarificationDetails;
		onReply: (sessionId: string, content: string) => Promise<void> | void;
		suggestions?: ClarificationSuggestion[];
	}

	let { sessionId, details, onReply, suggestions = [] }: Props = $props();

	let reply = $state('');
	let busy = $state(false);
	let errorMessage = $state<string | null>(null);
	let userRequestedReply = $state(false);

	let trimmedReply = $derived(reply.trim());
	let canSubmit = $derived(!busy && trimmedReply.length > 0);
	let hasSuggestions = $derived(suggestions.length > 0);
	let replyOpen = $derived(!hasSuggestions || userRequestedReply);

	function relativeTimestamp(iso: string): string {
		try {
			const asked = new Date(iso).getTime();
			const now = Date.now();
			const seconds = Math.max(0, Math.round((now - asked) / 1000));
			if (seconds < 60) return `${seconds}s ago`;
			const minutes = Math.round(seconds / 60);
			if (minutes < 60) return `${minutes}m ago`;
			const hours = Math.round(minutes / 60);
			return `${hours}h ago`;
		} catch {
			return '';
		}
	}

	async function dispatch(content: string) {
		busy = true;
		errorMessage = null;
		try {
			await onReply(sessionId, content);
			reply = '';
		} catch (err) {
			errorMessage = err instanceof Error ? err.message : String(err);
		} finally {
			busy = false;
		}
	}

	async function submit(event?: Event) {
		event?.preventDefault();
		if (!canSubmit) return;
		await dispatch(trimmedReply);
	}

	async function pick(suggestion: ClarificationSuggestion) {
		if (busy) return;
		await dispatch(suggestion.label);
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			void submit();
		}
		if (event.key === 'Escape' && hasSuggestions) {
			userRequestedReply = false;
		}
	}

	function openReply() {
		userRequestedReply = true;
	}

	function closeReply() {
		userRequestedReply = false;
		reply = '';
		errorMessage = null;
	}
</script>

<section
	class="clarification"
	aria-labelledby="clarification-question-{sessionId}"
	style="animation-delay: 0.12s"
>
	<header class="clarification-header">
		<span class="clarification-icon" aria-hidden="true">
			<MessageCircleQuestion size={14} />
		</span>
		<span class="clarification-sender">Agent asks</span>
		<span class="clarification-time" title={details.askedAt}>
			{relativeTimestamp(details.askedAt)}
		</span>
	</header>

	<p id="clarification-question-{sessionId}" class="clarification-question">
		{details.prompt}
	</p>

	{#if hasSuggestions}
		<div class="clarification-suggestions" role="group" aria-label="Suggested replies">
			{#each suggestions as suggestion, i (i)}
				<button
					type="button"
					class="suggestion-button"
					class:is-primary={suggestion.variant === 'primary'}
					disabled={busy}
					onclick={() => void pick(suggestion)}
				>
					{suggestion.label}
				</button>
			{/each}
			<span class="clarification-suggestions-spacer"></span>
			{#if !replyOpen}
				<button type="button" class="reply-trigger" disabled={busy} onclick={openReply}>
					Reply instead
				</button>
			{/if}
		</div>
	{/if}

	{#if replyOpen}
		<form class="clarification-form" onsubmit={submit}>
			<label class="visually-hidden" for="clarification-reply-{sessionId}">
				Reply to the agent's question
			</label>
			<textarea
				id="clarification-reply-{sessionId}"
				class="clarification-textarea"
				bind:value={reply}
				onkeydown={handleKeydown}
				placeholder="Answer to resume execution…"
				rows="3"
				disabled={busy}
				aria-describedby={errorMessage
					? `clarification-error-${sessionId}`
					: `clarification-hint-${sessionId}`}
			></textarea>

			{#if errorMessage}
				<p id="clarification-error-{sessionId}" class="clarification-error" role="alert">
					{errorMessage}
				</p>
			{:else}
				<p id="clarification-hint-{sessionId}" class="clarification-hint">
					Enter to send · Shift+Enter for a new line
				</p>
			{/if}

			<div class="clarification-actions">
				{#if hasSuggestions}
					<button type="button" class="reply-cancel" onclick={closeReply} disabled={busy}>
						Cancel
					</button>
				{/if}
				<button type="submit" class="reply-button" disabled={!canSubmit}>
					<Send size={12} aria-hidden="true" />
					<span>{busy ? 'Sending…' : 'Send reply'}</span>
				</button>
			</div>
		</form>
	{/if}
</section>

<style>
	.clarification {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3);
		margin: var(--space-2) var(--space-3);
		background: var(--color-surface);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		animation: clarification-fade-in 220ms var(--ease-out) backwards;
	}

	@keyframes clarification-fade-in {
		from {
			opacity: 0;
			transform: translateY(3px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	.clarification-header {
		display: flex;
		align-items: center;
		gap: var(--space-1-5);
	}

	.clarification-icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-tertiary);
	}

	.clarification-sender {
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-tertiary);
	}

	.clarification-time {
		margin-left: auto;
		font-size: var(--text-2xs);
		color: var(--color-text-muted);
	}

	.clarification-question {
		margin: 0;
		font-size: var(--text-sm);
		line-height: var(--leading-relaxed);
		color: var(--color-text-primary);
		white-space: pre-wrap;
		word-wrap: break-word;
		overflow-wrap: break-word;
	}

	.clarification-suggestions {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-1-5);
		padding-top: var(--space-1);
	}

	.clarification-suggestions-spacer {
		flex: 1;
	}

	.suggestion-button {
		appearance: none;
		padding: var(--space-1) var(--space-2-5);
		background: transparent;
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		font: inherit;
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-secondary);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			border-color var(--transition-fast),
			color var(--transition-fast);
	}

	.suggestion-button:hover:not(:disabled) {
		background: var(--color-surface-hover);
		border-color: var(--color-border-secondary);
		color: var(--color-text-primary);
	}

	.suggestion-button:focus-visible {
		outline: none;
		border-color: var(--color-accent-primary);
		box-shadow: 0 0 0 3px var(--color-primary-light);
	}

	.suggestion-button.is-primary {
		background: var(--color-accent-primary);
		border-color: var(--color-accent-primary);
		color: var(--color-text-on-accent, #fff);
	}

	.suggestion-button.is-primary:hover:not(:disabled) {
		background: var(--color-accent-primary-hover);
		border-color: var(--color-accent-primary-hover);
	}

	.suggestion-button:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
	}

	.reply-trigger {
		appearance: none;
		background: transparent;
		border: none;
		padding: var(--space-1) var(--space-2);
		font: inherit;
		font-size: var(--text-xs);
		color: var(--color-text-tertiary);
		cursor: pointer;
		border-radius: var(--radius-sm);
		transition:
			color var(--transition-fast),
			background var(--transition-fast);
	}

	.reply-trigger:hover:not(:disabled) {
		color: var(--color-text-primary);
		background: var(--color-interactive-hover);
	}

	.reply-trigger:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px var(--color-primary-light);
	}

	.clarification-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.clarification-textarea {
		resize: vertical;
		min-height: calc(var(--space-6) * 2);
		padding: var(--space-2);
		font-family: inherit;
		font-size: var(--text-sm);
		line-height: var(--leading-normal);
		color: var(--color-text-primary);
		background: var(--color-bg-base);
		border: 1px solid var(--color-border-primary);
		border-radius: var(--radius-sm);
		transition:
			border-color var(--transition-fast),
			box-shadow var(--transition-fast);
	}

	.clarification-textarea:focus-visible {
		outline: none;
		border-color: var(--color-accent-primary);
		box-shadow: 0 0 0 3px var(--color-primary-light);
	}

	.clarification-textarea:disabled {
		opacity: var(--opacity-muted);
		cursor: not-allowed;
	}

	.clarification-hint,
	.clarification-error {
		margin: 0;
		font-size: var(--text-2xs);
	}

	.clarification-hint {
		color: var(--color-text-tertiary);
	}

	.clarification-error {
		color: var(--color-error);
	}

	.clarification-actions {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-1-5);
	}

	.reply-cancel {
		appearance: none;
		background: transparent;
		border: none;
		padding: var(--space-1) var(--space-2);
		font: inherit;
		font-size: var(--text-xs);
		color: var(--color-text-tertiary);
		cursor: pointer;
		border-radius: var(--radius-sm);
		transition: color var(--transition-fast);
	}

	.reply-cancel:hover:not(:disabled) {
		color: var(--color-text-primary);
	}

	.reply-cancel:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
	}

	.reply-button {
		display: inline-flex;
		align-items: center;
		gap: var(--space-1-5);
		padding: var(--space-1) var(--space-2-5);
		font-size: var(--text-xs);
		font-weight: var(--font-medium);
		color: var(--color-text-on-accent, #fff);
		background: var(--color-accent-primary);
		border: 1px solid var(--color-accent-primary);
		border-radius: var(--radius-sm);
		cursor: pointer;
		transition:
			background var(--transition-fast),
			border-color var(--transition-fast),
			opacity var(--transition-fast);
	}

	.reply-button:hover:not(:disabled) {
		background: var(--color-accent-primary-hover);
		border-color: var(--color-accent-primary-hover);
	}

	.reply-button:focus-visible {
		outline: none;
		box-shadow: 0 0 0 3px var(--color-primary-light);
	}

	.reply-button:disabled {
		opacity: var(--opacity-disabled);
		cursor: not-allowed;
	}

	.visually-hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	@media (prefers-reduced-motion: reduce) {
		.clarification {
			animation: none;
		}
		.suggestion-button,
		.reply-button,
		.reply-trigger,
		.reply-cancel,
		.clarification-textarea {
			transition: none;
		}
	}
</style>
