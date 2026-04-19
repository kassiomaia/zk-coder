/**
 * Agent status phrase store.
 *
 * Separated from the main agent store so high-frequency `StatusPhrase`
 * events (~25/step during LLM/tool phases) no longer cascade across
 * every subscriber of the sessions Svelte store. Consumers that need
 * the phrases — the `AnsweringIndicator` via `getThreadAgentState` —
 * read from here directly; other session-store subscribers never see
 * these updates.
 */

import { SvelteMap } from 'svelte/reactivity';

/** Maximum phrases retained per session for rotation in the indicator. */
const MAX_PHRASES_PER_SESSION = 10;

/** Keyed by sessionId. Reactive reads in a `$derived` block re-run only
 *  when the accessed key's value changes, keeping the status pathway
 *  isolated from unrelated session events. */
export const agentStatusPhrases = new SvelteMap<string, string[]>();

/** Append a phrase for `sessionId`, keeping only the most recent
 *  {@link MAX_PHRASES_PER_SESSION}. */
export function appendStatusPhrase(sessionId: string, phrase: string): void {
	const existing = agentStatusPhrases.get(sessionId) ?? [];
	const next = [...existing, phrase].slice(-MAX_PHRASES_PER_SESSION);
	agentStatusPhrases.set(sessionId, next);
}

/** Drop all phrases for a session — called when the session resets or
 *  is cleared from the agent store. */
export function clearStatusPhrases(sessionId: string): void {
	agentStatusPhrases.delete(sessionId);
}

/** Read-only helper that returns the current phrases for a session, or
 *  `undefined` if none have been recorded. */
export function getStatusPhrases(sessionId: string): string[] | undefined {
	return agentStatusPhrases.get(sessionId);
}
