/**
 * Runtime configuration constants.
 *
 * Centralized home for timing, buffering, and retention numbers that would
 * otherwise live as magic numbers inside stores and utilities. Keep this file
 * value-only — no behavior.
 */

/**
 * Maximum time between streaming chunks before a stream is considered inactive
 * and cancelled. Large enough to accommodate slow providers/models; total wait
 * is still bounded by the backend request timeout and explicit cancellation.
 */
export const STREAM_INACTIVITY_TIMEOUT_MS = 150_000;

/**
 * Upper bound on events buffered while a stream is waiting to learn its
 * `requestId`. Older events are dropped first.
 */
export const STREAM_MAX_BUFFERED_EVENTS = 300;

/**
 * Default retention window for completed document-generation jobs. Jobs older
 * than this are eligible for `clearOldJobs`.
 */
export const DOC_GEN_MAX_AGE_MS = 5 * 60 * 1000;

/**
 * Default debounce for NLP classification triggered from chat input.
 */
export const NLP_DEBOUNCE_MS = 300;
