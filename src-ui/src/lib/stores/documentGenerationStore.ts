/**
 * Document Generation Store
 *
 * Reactive store for tracking active document generation jobs.
 * Provides a centralized state that components can subscribe to
 * for real-time updates on document generation progress.
 */

import { derived, writable } from 'svelte/store';
import type { DocumentGenerationJob } from '$lib/utils/documentGenerationTracker';
import { documentGenerationTracker } from '$lib/utils/documentGenerationTracker';

// ============================================================================
// Types
// ============================================================================

/** Store state for document generation */
export interface DocumentGenerationState {
	jobs: DocumentGenerationJob[];
	activeJob: DocumentGenerationJob | null;
	hasActiveJob: boolean;
}

// ============================================================================
// Store Factory
// ============================================================================

function createDocumentGenerationStore() {
	const { subscribe, update } = writable<DocumentGenerationState>({
		jobs: [],
		activeJob: null,
		hasActiveJob: false
	});

	// Refresh state from the tracker. The store surfaces the latest job —
	// complete or not — so the card stays visible through the "Ready" state
	// until the user dismisses it or it ages out via `clearOldJobs`.
	// `hasActiveJob` still reflects only non-complete jobs, for callers that
	// gate work on "is generation in flight?".
	function refresh() {
		const all = documentGenerationTracker
			.getAllJobs()
			.sort((a, b) => b.updatedAt.getTime() - a.updatedAt.getTime());
		const latest = all[0] ?? null;
		const inFlight = all.some((j) => !j.complete);

		update((state) => ({
			...state,
			jobs: all,
			activeJob: latest,
			hasActiveJob: inFlight
		}));
	}

	// Process a message and update state if relevant
	function processMessage(message: import('$lib/types/message').Message) {
		const job = documentGenerationTracker.processMessage(message);
		if (job) {
			refresh();
			return job;
		}
		return null;
	}

	// Get job by session ID
	function getJobBySession(sessionId: string): DocumentGenerationJob | undefined {
		return documentGenerationTracker.getJobBySession(sessionId);
	}

	// Get job by ID
	function getJob(jobId: string): DocumentGenerationJob | undefined {
		return documentGenerationTracker.getJob(jobId);
	}

	// Retry a failed job
	function retryJob(jobId: string): boolean {
		const success = documentGenerationTracker.retryJob(jobId);
		if (success) {
			refresh();
		}
		return success;
	}

	// Remove/dismiss a job
	function removeJob(jobId: string): boolean {
		const success = documentGenerationTracker.removeJob(jobId);
		if (success) {
			refresh();
		}
		return success;
	}

	// Clear old completed jobs
	function clearOldJobs(maxAgeMs: number = 5 * 60 * 1000) {
		documentGenerationTracker.clearOldJobs(maxAgeMs);
		refresh();
	}

	// Reset all state
	function reset() {
		documentGenerationTracker.clearOldJobs(0); // Clear all jobs
		refresh();
	}

	return {
		subscribe,
		refresh,
		processMessage,
		getJobBySession,
		getJob,
		retryJob,
		removeJob,
		clearOldJobs,
		reset
	};
}

// ============================================================================
// Store Instance
// ============================================================================

export const documentGenerationStore = createDocumentGenerationStore();

// ============================================================================
// Derived Stores
// ============================================================================

/**
 * Store for the currently active document generation job
 */
export const activeDocGenJob = derived(documentGenerationStore, ($state) => $state.activeJob);

/**
 * Store for whether there's an active document generation job
 */
export const hasActiveDocGenJob = derived(documentGenerationStore, ($state) => $state.hasActiveJob);

/**
 * Store for all active document generation jobs
 */
export const activeDocGenJobs = derived(documentGenerationStore, ($state) => $state.jobs);
