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
	tasks: AgentTask[];
	filesTouched: TouchedFile[];
	terminalOutput: TerminalLine[];
	createdAt: string;
	updatedAt: string;
}

/**
 * Session lifecycle state (discriminated union)
 */
export type SessionState =
	| { type: 'launching' }
	| { type: 'running'; currentStep: number; maxSteps: number }
	| { type: 'wait_for_request'; lastActionAt: string }
	| { type: 'user_input_required'; pendingActions: PendingAction[] }
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
}

/**
 * Terminal output line
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
	| { type: 'execution_complete'; sessionId: string; result: ExecutionResult }
	| { type: 'execution_error'; sessionId: string; error: string }
	| { type: 'approval_required'; sessionId: string; action: PendingAction }
	| { type: 'approval_approved'; sessionId: string; actionId: string }
	| { type: 'approval_denied'; sessionId: string; actionId: string }
	| { type: 'command_output'; sessionId: string; stream: 'stdout' | 'stderr'; content: string }
	| { type: 'command_finished'; sessionId: string; exitCode: number | null }
	| { type: 'subtask_created'; sessionId: string; subtask: SubTask }
	| { type: 'subtask_updated'; sessionId: string; subtask: SubTask };

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
