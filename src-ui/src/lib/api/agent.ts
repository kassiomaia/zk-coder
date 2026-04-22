/**
 * Agent API client.
 *
 * Typed wrappers for Tauri IPC commands in the agent domain that components
 * should use instead of calling `invoke()` directly.
 */

import { invokeCommand } from './core';

export { AgentApiError } from './errors';

/**
 * Open the agent trace/debug window for a session. Only meaningful in dev
 * builds; the caller is responsible for gating on `import.meta.env.DEV`.
 */
export async function openAgentTraceWindow(sessionId: string): Promise<void> {
	await invokeCommand<void>('agent', 'open_agent_trace_window', { sessionId });
}
