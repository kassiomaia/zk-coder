/**
 * Agent Types
 * Type definitions for agent orchestration in the frontend.
 * Maps to Rust backend types in src-tauri/src/agents/.
 */

/** Session identifier */
export type SessionId = string;

/** Agent identifier */
export type AgentId = string;

/** Trust mode determines permission behavior */
export type TrustMode = 'trusted' | 'untrusted' | 'restricted';

/** Task execution status */
export type TaskStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled';

/** Permission level for tools */
export type PermissionLevel = 'auto_run' | 'approval_required' | 'disabled';

/**
 * Agent configuration (persisted)
 */
export interface Agent {
	id: AgentId;
	name: string;
	description: string;
	systemPrompt: string;
	allowedTools: string[];
	defaultModelId: string;
	trustMode: TrustMode;
}

/**
 * Agent session (runtime instance)
 */
export interface AgentSession {
	id: SessionId;
	agentId: AgentId;
	agentName: string;
	conversationId: string;
	threadId: string;
	state: SessionState;
	workspaceRoot?: string;
	tasks?: AgentTask[];
	filesTouched?: TouchedFile[];
	terminalOutput?: TerminalLine[];
	toolCalls?: ToolCallRecord[];
	executionSteps?: ExecutionStep[]; // Track execution steps for UI progress display
	statusPhrases?: string[]; // Accumulated status phrases for rotation
	streamingAssistantContent?: string; // Accumulated content for streaming assistant message
	createdAt: string;
	updatedAt: string;
}

/**
 * Session lifecycle state (discriminated union)
 */
export type SessionState =
	| { type: 'launching' }
	| { type: 'running'; currentStep: number; contextUsage: number }
	| { type: 'wait_for_request'; lastActionAt: string }
	| { type: 'user_input_required'; pendingActionIds: string[] }
	| { type: 'finished'; completedAt: string }
	| { type: 'cancelled'; reason: string }
	| { type: 'error'; message: string; recoverable: boolean };

/**
 * Task tracked by the agent
 */
export interface AgentTask {
	id: string;
	title: string;
	description?: string;
	status: TaskStatus;
	selected?: boolean;
	subtasks?: SubTask[];
}

/**
 * Raw task shape from the Rust backend (used in events and API responses).
 * Maps to frontend AgentTask via toAgentTask().
 */
export interface BackendAgentTask {
	id: string;
	sessionId: string;
	request: { content: string };
	status: TaskStatus;
	steps: unknown[];
	createdAt: string;
	completedAt?: string;
}

/** Convert a backend task to the frontend AgentTask shape. */
export function toAgentTask(bt: BackendAgentTask): AgentTask {
	// Use the first line of the request as the title, full content as description
	const content = bt.request.content;
	const firstLine = content.split('\n')[0].slice(0, 80);
	return {
		id: bt.id,
		title: firstLine || 'Task',
		description: content.length > 80 ? content : undefined,
		status: bt.status,
		subtasks: []
	};
}

/**
 * A subtask within an agent's task decomposition
 */
export interface SubTask {
	id: string;
	taskId: string;
	title: string;
	description?: string;
	status: TaskStatus;
	order: number;
	createdAt: string;
	completedAt?: string;
}

/**
 * File touched by the agent during execution
 */
export interface TouchedFile {
	path: string;
	modified: boolean;
	action: 'read' | 'write' | 'delete';
	/** Unified diff content for write operations (available after approval) */
	diff?: string;
}

/**
 * Tool call record (for technical debugging/tracing)
 */
export interface ToolCallRecord {
	id: string;
	tool: string;
	arguments: Record<string, any>;
	result?: Record<string, any>;
	isError?: boolean;
	durationMs?: number;
	timestamp: string;
}

/**
 * Terminal output line (Legacy/Stream output)
 */
export interface TerminalLine {
	id: string;
	type: 'stdout' | 'stderr' | 'command' | 'info';
	content: string;
	timestamp?: string;
}

/**
 * Action awaiting user approval
 */
export interface PendingAction {
	id: string;
	toolName: string;
	actionType: ActionType;
	description: string;
	preview?: ActionPreview;
	arguments: Record<string, unknown>;
	createdAt: string;
}

/**
 * Type of action being performed (discriminated union)
 */
export type ActionType =
	| { type: 'read_file'; path: string }
	| { type: 'write_file'; path: string; sizeBytes: number }
	| { type: 'delete_file'; path: string }
	| { type: 'execute_command'; command: string }
	| { type: 'web_request'; url: string };

/**
 * Preview of action content (for approval dialog)
 */
export interface ActionPreview {
	contentType: 'diff' | 'text' | 'json';
	content: string;
	language?: string;
}

/**
 * Agent event from backend (via Tauri events)
 */
export type AgentEvent =
	| { type: 'session_created'; sessionId: string; agentId: string }
	| { type: 'session_state_changed'; sessionId: string; state: SessionState }
	| { type: 'execution_started'; sessionId: string; requestId: string }
	| { type: 'execution_step'; sessionId: string; step: ExecutionStep }
	| {
			type: 'execution_complete';
			sessionId: string;
			conversationId: string;
			threadId: string;
			result: ExecutionResult;
	  }
	| { type: 'execution_error'; sessionId: string; error: string }
	| { type: 'approval_required'; sessionId: string; action: PendingAction }
	| { type: 'approval_approved'; sessionId: string; actionId: string }
	| { type: 'approval_denied'; sessionId: string; actionId: string }
	| { type: 'command_output'; sessionId: string; stream: 'stdout' | 'stderr'; content: string }
	| { type: 'command_finished'; sessionId: string; exitCode: number | null }
	| {
			type: 'tool_call';
			sessionId: string;
			tool: string;
			arguments: Record<string, unknown>;
	  }
	| {
			type: 'tool_result';
			sessionId: string;
			tool: string;
			result: Record<string, unknown>;
			isError: boolean;
	  }
	| { type: 'task_created'; sessionId: string; task: BackendAgentTask }
	| { type: 'task_updated'; sessionId: string; taskId: string; status: TaskStatus }
	| { type: 'subtask_created'; sessionId: string; subtask: SubTask }
	| { type: 'subtask_updated'; sessionId: string; subtask: SubTask }
	| {
			type: 'file_touched';
			sessionId: string;
			filePath: string;
			action: FileAction;
			/** Unified diff of changes (only present for write actions) */
			diff?: string;
	  }
	| { type: 'status_phrase'; sessionId: string; message: string; timestamp: string }
	| {
		type: 'streaming_content';
		sessionId: string;
		conversationId: string;
		threadId: string;
		chunk: string;
		isFinal: boolean;
	  }
	| {
		type: 'context_compacted';
		sessionId: string;
		conversationId: string;
		beforeCount: number;
		afterCount: number;
		beforeTokens: number;
		afterTokens: number;
		timestamp: string;
	};

/** Action performed on a file */
export type FileAction = 'read' | 'write' | 'delete';

/**
 * Single step in agent execution
 */
export interface ExecutionStep {
	stepNumber: number;
	action: StepAction;
	result?: StepResult;
	timestamp: string;
}

/** Step action type */
export type StepAction =
	| { type: 'llm_request'; model: string }
	| { type: 'tool_call'; tool: string; arguments: Record<string, unknown> };

/** Step result */
export interface StepResult {
	success: boolean;
	output: Record<string, unknown>;
	tokensUsed?: number;
	durationMs: number;
}

/**
 * Full execution result
 */
export interface ExecutionResult {
	content: string;
	steps: ExecutionStep[];
	usage?: {
		inputTokens: number;
		outputTokens: number;
		totalTokens: number;
	};
}
