/**
 * Main Event Handler
 * Central event dispatcher for agent events.
 */

import type {AgentEvent} from '$lib/types/agent';
import type {AgentState} from '../state';
import * as SessionEvents from './sessionEvents';
import * as ToolEvents from './toolEvents';
import * as TerminalEvents from './terminalEvents';
import * as TaskEvents from './taskEvents';
import * as FileEvents from './fileEvents';
import * as StreamingEvents from './streamingEvents';

/**
 * Handle an agent event from the backend.
 */
export function handleAgentEvent(state: AgentState, event: AgentEvent): AgentState {
	console.debug('[agentStore] Received event:', event.type, event);

	switch (event.type) {
		case 'session_state_changed':
			SessionEvents.handleSessionStateChanged(state, event);
			break;
		case 'approval_required':
			SessionEvents.handleApprovalRequired(state, event);
			break;
		case 'approval_approved':
			SessionEvents.handleApprovalApproved(state, event);
			break;
		case 'approval_denied':
			SessionEvents.handleApprovalDenied(state, event);
			break;
		case 'execution_error':
			SessionEvents.handleExecutionError(state, event);
			break;
		case 'command_output':
			TerminalEvents.handleCommandOutput(state, event);
			break;
		case 'command_finished':
			TerminalEvents.handleCommandFinished(state, event);
			break;
		case 'task_created':
			TaskEvents.handleTaskCreated(state, event);
			break;
		case 'task_updated':
			TaskEvents.handleTaskUpdated(state, event);
			break;
		case 'subtask_created':
			TaskEvents.handleSubtaskCreated(state, event);
			break;
		case 'subtask_updated':
			TaskEvents.handleSubtaskUpdated(state, event);
			break;
		case 'file_touched':
			FileEvents.handleFileTouched(state, event);
			break;
		case 'execution_started':
			SessionEvents.handleExecutionStarted(state, event);
			break;
		case 'execution_step':
			SessionEvents.handleExecutionStep(state, event);
			break;
		case 'tool_call':
			ToolEvents.handleToolCall(state, event);
			break;
		case 'tool_result':
			ToolEvents.handleToolResult(state, event);
			break;
		case 'execution_complete':
			SessionEvents.handleExecutionComplete(state, event);
			break;
		case 'status_phrase':
			StreamingEvents.handleStatusPhrase(state, event);
			break;
		case 'agent_narration':
			StreamingEvents.handleAgentNarration(state, event);
			break;
		case 'streaming_content':
			StreamingEvents.handleStreamingContent(state, event);
			break;
		case 'context_compacted':
			StreamingEvents.handleContextCompacted(state, event);
			break;
		case 'artifact_created':
			SessionEvents.handleArtifactCreated(state, event);
			break;
	}

	// Update the session's updatedAt timestamp for stall detection
	// This applies to all events that reference a valid session
	const session = state.sessions.get(event.sessionId);
	if (session) {
		state.sessions.set(event.sessionId, {
			...session,
			updatedAt: new Date().toISOString()
		});
	}

	return { ...state, sessions: new Map(state.sessions) };
}
