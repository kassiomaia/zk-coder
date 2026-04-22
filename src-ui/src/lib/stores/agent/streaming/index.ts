/**
 * Streaming Module Index
 * Exports all streaming functionality.
 *
 * Both submodules emit `agent-activity` events on the shared
 * `agentEventBus`; no explicit wire-up step is required.
 */

export {
	flushStreamingUpdate,
	scheduleStreamingUpdate,
	clearStreamingUpdates,
	STREAM_THROTTLE_MS
} from './streamingThrottle';

export {
	flushTerminalOutput,
	scheduleTerminalOutput,
	flushPendingTerminalOutput,
	clearTerminalOutput,
	TERMINAL_BATCH_MS
} from './terminalBatch';
