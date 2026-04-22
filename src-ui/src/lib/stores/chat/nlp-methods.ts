/**
 * NLP methods extracted from the chat store during Phase 7 of the SOLID
 * refactor. The factory builds a thin object of methods bound to the
 * store's `update` / `getState` closures so the public API keeps working
 * without the chat store owning the NLP implementation details.
 */

import {
	applyResolutions,
	createDebouncedClassifier,
	detectAmbiguities,
	resolveAmbiguity
} from '$lib/api/nlp';
import type { ResolutionType } from '$lib/types/nlp';
import type { ChatState } from './types';
import { createLogger } from '$lib/utils/logger';

type Updater = (fn: (state: ChatState) => ChatState) => void;
type GetState = () => ChatState;
type DebouncedClassifier = ReturnType<typeof createDebouncedClassifier>;

interface NlpMethodDeps {
	update: Updater;
	getState: GetState;
	classifier: DebouncedClassifier;
}

const log = createLogger('chatStore.nlp');

export function createNlpMethods({ update, getState, classifier }: NlpMethodDeps) {
	async function classifyInput(input: string) {
		const state = getState();
		if (!state.nlp.backendAvailable || !state.selectedConversationId) return;

		update((s) => ({ ...s, nlp: { ...s.nlp, processing: true, error: null } }));

		try {
			const result = await classifier.classify({
				input,
				conversationId: state.selectedConversationId
			});

			update((s) => ({
				...s,
				nlp: { ...s.nlp, detectedIntent: result, processing: false }
			}));
		} catch (error) {
			update((s) => ({
				...s,
				nlp: {
					...s.nlp,
					processing: false,
					error: error instanceof Error ? error.message : 'Classification failed'
				}
			}));
		}
	}

	async function detectAmbiguitiesForInput(input: string) {
		const state = getState();
		if (!state.nlp.backendAvailable || !state.selectedConversationId) return;

		try {
			const ambiguities = await detectAmbiguities({
				input,
				conversationId: state.selectedConversationId
			});
			update((s) => ({
				...s,
				nlp: {
					...s.nlp,
					ambiguities,
					// Auto-populate pending ambiguities for high/medium severity
					pendingAmbiguities: ambiguities.filter((a) => a.severity !== 'low')
				}
			}));
		} catch (error) {
			log.error('Failed to detect ambiguities:', error);
		}
	}

	function clearNlpState() {
		classifier.cancel();
		update((state) => ({
			...state,
			nlp: {
				...state.nlp,
				detectedIntent: null,
				ambiguities: [],
				pendingAmbiguities: [],
				resolvedAmbiguities: new Map(),
				processing: false,
				error: null
			}
		}));
	}

	async function resolveAmbiguityById(
		ambiguityId: string,
		resolution: string,
		resolutionType: ResolutionType
	): Promise<boolean> {
		const state = getState();
		const ambiguity = state.nlp.pendingAmbiguities.find((a) => a.id === ambiguityId);

		if (!ambiguity) {
			update((s) => ({
				...s,
				nlp: { ...s.nlp, error: 'Ambiguity not found' }
			}));
			return false;
		}

		try {
			const resolved = await resolveAmbiguity({
				ambiguityId,
				originalText: ambiguity.span.text,
				resolution,
				resolutionType
			});

			update((s) => {
				const newResolvedAmbiguities = new Map(s.nlp.resolvedAmbiguities);
				newResolvedAmbiguities.set(ambiguityId, resolved);

				return {
					...s,
					nlp: {
						...s.nlp,
						pendingAmbiguities: s.nlp.pendingAmbiguities.filter((a) => a.id !== ambiguityId),
						resolvedAmbiguities: newResolvedAmbiguities
					}
				};
			});

			return true;
		} catch (error) {
			update((s) => ({
				...s,
				nlp: {
					...s.nlp,
					error: error instanceof Error ? error.message : 'Failed to resolve ambiguity'
				}
			}));
			return false;
		}
	}

	async function dismissAmbiguity(ambiguityId: string): Promise<boolean> {
		return resolveAmbiguityById(ambiguityId, '', 'dismissed');
	}

	function hasBlockingAmbiguities(): boolean {
		const state = getState();
		return state.nlp.pendingAmbiguities.some((a) => a.severity === 'high');
	}

	async function applyAmbiguityResolutions(input: string): Promise<string> {
		const state = getState();
		const resolutions: Record<string, string> = {};

		state.nlp.resolvedAmbiguities.forEach((resolved, id) => {
			if (resolved.resolution_type !== 'dismissed') {
				resolutions[id] = resolved.resolution;
			}
		});

		if (Object.keys(resolutions).length === 0) {
			return input;
		}

		try {
			return await applyResolutions({ input, resolutions });
		} catch (error) {
			log.error('Failed to apply resolutions:', error);
			return input;
		}
	}

	function clearResolvedAmbiguities() {
		update((state) => ({
			...state,
			nlp: {
				...state.nlp,
				resolvedAmbiguities: new Map()
			}
		}));
	}

	return {
		classifyInput,
		detectAmbiguitiesForInput,
		clearNlpState,
		resolveAmbiguity: resolveAmbiguityById,
		dismissAmbiguity,
		hasBlockingAmbiguities,
		applyAmbiguityResolutions,
		clearResolvedAmbiguities
	};
}
