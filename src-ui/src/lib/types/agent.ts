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
export type TaskStatus =
	| 'pending'
	| 'in_progress'
	| 'completed'
	| 'failed'
	| 'cancelled'
	| 'aborted';

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

/** Reference to a document artifact created by an agent. */
export interface ArtifactReference {
	id: string;
	sessionId: string;
	artifactType: 'Docx' | 'Pptx' | 'Pdf';
	filename: string;
	sizeBytes: number;
	createdAt: string;
}

/**
 * Narration intent (Workstream E.1).
 *
 * Mirrors the Rust `NarrationIntent` enum. `question` signals that the
 * harness is paused until the user replies; AgentPanel renders those
 * inline with a reply composer. `narration` is informational.
 */
export type NarrationIntent = 'narration' | 'question';

/**
 * Long-running harness role for a session window.
 *
 * `awaiting_clarification` (Workstream E.2) is the paused state the
 * runtime enters when the agent emits `agent_narrate` with
 * `intent = "question"`. The accompanying prompt + timestamp travel on
 * {@link AgentSessionHarnessState.awaitingClarification} so the flat
 * string shape remains compatible with existing equality checks.
 */
export type HarnessMode =
	| 'initializer'
	| 'worker'
	| 'resume_verifier'
	| 'awaiting_clarification';

/**
 * Accompaniment to the `awaiting_clarification` harness mode
 * (Workstream E.2). Present when the backend has paused the runtime
 * awaiting a user reply; undefined otherwise.
 */
export interface AwaitingClarificationDetails {
	prompt: string;
	askedAt: string;
}

/** Startup verification lifecycle state. */
export type StartupVerificationState = 'idle' | 'running' | 'passed' | 'failed';

/** Environment health summary for session handoff. */
export type EnvironmentHealth = 'unknown' | 'healthy' | 'degraded' | 'blocked';

/** Session-facing startup verification summary. */
export interface StartupVerificationSummary {
	state: StartupVerificationState;
	headline: string;
	details?: string[];
	checkedAt?: string;
}

/** Session-facing handoff summary for long-running work. */
export interface SessionHandoffSummary {
	lastCompletedIncrement: string;
	currentObjective: string;
	nextRecommendedAction: string;
	openIssues: string[];
	environmentHealth: EnvironmentHealth;
	recordedAt?: string;
}

/** Additive harness metadata grouped beside coarse session lifecycle state. */
export interface AgentSessionHarnessState {
	mode: HarnessMode;
	startupVerification?: StartupVerificationSummary;
	handoff?: SessionHandoffSummary;
	/**
	 * Present when `mode === 'awaiting_clarification'` (Workstream E.2).
	 * Carries the exact question the agent emitted together with the
	 * timestamp the question was asked at. AgentPanel renders the
	 * question inline with a reply composer.
	 */
	awaitingClarification?: AwaitingClarificationDetails;
	cleanExitRecorded: boolean;
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
	modelId?: string;
	tasks?: AgentTask[];
	filesTouched?: TouchedFile[];
	/** Immediate children of the workspace root, shown in the Files
	 *  panel. Populated via `loadWorkspaceRoot` on workspace change.
	 *  Deeper directories load on demand when the user expands them. */
	workspaceRootEntries?: WorkspaceEntry[];
	/** True when the root directory contained more entries than the
	 *  backend cap and the listing is partial. */
	workspaceRootTruncated?: boolean;
	terminalOutput?: TerminalLine[];
	toolCalls?: ToolCallRecord[];
	executionSteps?: ExecutionStep[]; // Track execution steps for UI progress display
	streamingMessages?: Record<string, string>; // Accumulated content for active streaming assistant messages
	artifacts?: ArtifactReference[]; // Artifacts created during this session
	harnessState?: AgentSessionHarnessState; // Long-running harness metadata
	/**
	 * Workstream F.2 — `true` when the model marked at least one
	 * subtask `Failed` via `agent_mark_subtask_status`. Persists
	 * across reload (`agent_sessions.needs_review` column). UI may
	 * surface a post-run review affordance when set.
	 */
	needsReview?: boolean;
	createdAt: string;
	updatedAt: string;
}

export type ExecutionPhase =
	| 'bootstrapping'
	| 'preparing_step'
	| 'resolving_context'
	| 'calling_llm'
	| 'handling_llm_response'
	| 'executing_tools'
	| 'completed'
	| 'failed'
	| 'cancelled';

export interface ExecutionSnapshot {
	sessionId: string;
	taskId: string;
	stepNumber: number;
	phase: ExecutionPhase;
	currentSubtaskId?: string | null;
	contextUsagePercent: number;
	totalInputTokens: number;
	totalOutputTokens: number;
	lastModel?: string | null;
	lastError?: string | null;
	updatedAt: string;
}

export type RuntimeTraceType =
	| 'execution_started'
	| 'step_started'
	| 'step_finished'
	| 'phase_transition'
	| 'subtask_context_resolved'
	| 'retrieval_started'
	| 'retrieval_completed'
	| 'context_policy_evaluated'
	| 'toolset_filtered'
	| 'llm_request_prepared'
	| 'llm_request_started'
	| 'llm_retry_scheduled'
	| 'llm_response_received'
	| 'llm_response_classified'
	| 'tool_batch_started'
	| 'tool_batch_completed'
	| 'execution_cancelled'
	| 'execution_failed'
	| 'execution_completed'
	| 'subtask_scope_drift'
	| 'clarification_replied'
	| 'clarification_resumed';

export interface RuntimeTrace {
	id: string;
	sessionId: string;
	taskId: string;
	subtaskId?: string | null;
	stepNumber: number;
	traceType: RuntimeTraceType;
	payloadJson: Record<string, unknown> | unknown[] | string | number | boolean | null;
	createdAt: string;
}

export type AgentTraceEvent =
	| { type: 'runtime_trace'; trace: RuntimeTrace }
	| { type: 'snapshot_updated'; snapshot: ExecutionSnapshot };

export type PauseReason = 'startup_recovery' | 'clarification_timeout' | 'user_initiated';

/**
 * Session lifecycle state (discriminated union)
 */
export type SessionState =
	| { type: 'launching' }
	| { type: 'running'; currentStep: number; contextUsage: number }
	| { type: 'wait_for_request'; lastActionAt: string }
	| { type: 'user_input_required'; pendingActionIds: string[] }
	| {
			type: 'paused';
			pausedAt: string;
			pauseReason: PauseReason;
			message: string;
			currentSubtaskId?: string;
	  }
	| {
			type: 'awaiting_clarification';
			prompt: string;
			askedAt: string;
			subtaskId?: string;
	  }
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
	isNonTrivial?: boolean;
}

/**
 * Raw task shape from the Rust backend (used in events and API responses).
 * Maps to frontend AgentTask via toAgentTask().
 */
export interface BackendAgentTask {
	id: string;
	sessionId: string;
	request: { content: string };
	title?: string; // LLM-generated title from single-shot planning
	description?: string; // LLM-generated description
	status: TaskStatus;
	steps: unknown[];
	subtasks?: SubTask[];
	/**
	 * Task-declared framing (Workstream H.2). Present when the
	 * planning step supplied a `framing` record with role, voice
	 * constraints, optional domain priors, and optional forbidden
	 * patterns. Legacy tasks that rely on the default code framing
	 * leave this undefined. No UI rendering is required in this
	 * phase; the field is exposed so a future override surface can
	 * read from it without another type migration.
	 */
	framing?: TaskFraming;
	createdAt: string;
	completedAt?: string;
	needsReview?: boolean; // Flag for tasks that stopped mid-execution
}

/**
 * Structured domain priors carried alongside {@link TaskFraming}
 * (Workstream H.2). v1 is a free-form bulleted hint list; the shape
 * is intentionally permissive so the planner can declare hints today
 * without blocking on the richer schema.
 */
export interface DomainPriors {
	hints: string[];
}

/**
 * Task-declared framing (Workstream H.2).
 *
 * Mirrors the Rust `TaskFraming` record. Supplied by the planner with
 * the `agent_plan_subtasks` call and stable for the duration of the
 * task. See `docs/planning/agent-fidelity/system-prompt-architecture.md`
 * for the design intent.
 */
export interface TaskFraming {
	role: string;
	voiceConstraints: string[];
	domainPriors?: DomainPriors;
	forbiddenPatterns?: string[];
}

/** Convert a backend task to the frontend AgentTask shape. */
export function toAgentTask(bt: BackendAgentTask): AgentTask {
	// Use the backend-provided title if available (from LLM planning)
	// Otherwise fall back to truncating the request content (legacy behavior)
	if (bt.title) {
		return {
			id: bt.id,
			title: bt.title,
			description: bt.description,
			status: bt.status,
			subtasks: bt.subtasks ?? [],
			selected: false
		};
	}

	// Legacy fallback: truncate first line
	const content = bt.request.content;
	const firstLine = content.split('\n')[0].slice(0, 80);
	return {
		id: bt.id,
		title: firstLine || 'Task',
		description: content.length > 80 ? content : undefined,
		status: bt.status,
		subtasks: bt.subtasks ?? [],
		selected: false
	};
}

/**
 * Subtask execution mode (determines what tools are available)
 */
export type SubTaskMode = 'read' | 'write' | 'execute' | 'full_access';

/**
 * Artifact kind declared on a {@link ArtifactSpec} (Workstream B.1).
 *
 * Mirrors the Rust `ArtifactKind` enum in `core/agent/task.rs`. Used by
 * scope-drift detection to label the expected output at planning time.
 */
export type ArtifactKind = 'file' | 'document' | 'command' | 'other';

/**
 * A single artifact expectation on a subtask contract.
 *
 * `pathPattern` is a glob-style pattern (e.g. `docs/social/*.md`,
 * `reports/**​/*.pdf`). The backend uses `globset` to match written
 * paths against the pattern; non-matching writes surface as a
 * `subtask_scope_drift` runtime trace.
 */
export interface ArtifactSpec {
	pathPattern: string;
	kind: ArtifactKind;
}

/**
 * The contract a subtask declares at planning time (Workstream B.1).
 *
 * When present, write-class tool calls whose output path does not
 * match any `expectedArtifacts` entry are flagged as scope drift in
 * the next step's context. The field is optional — legacy plans
 * carry no contract and preserve pre-B behavior (no detection).
 */
export interface SubTaskContract {
	expectedArtifacts: ArtifactSpec[];
}

/**
 * A subtask within an agent's task decomposition
 */
export interface SubTask {
	id: string;
	taskId: string;
	title: string;
	description?: string;
	mode: SubTaskMode;
	status: TaskStatus;
	order: number;
	/**
	 * Optional contract describing the artifacts this subtask is
	 * expected to produce (Workstream B.1). Absent on legacy plans;
	 * the agent panel does not render it yet — contract-aware UI
	 * arrives with H.3.
	 */
	contract?: SubTaskContract;
	createdAt: string;
	startedAt?: string;
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
 * One entry in the workspace file listing that powers the Files panel.
 * Paths are relative to `AgentSession.workspaceRoot` with forward-slash
 * separators.
 */
export interface WorkspaceEntry {
	path: string;
	kind: 'file' | 'directory';
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
	| { type: 'web_request'; url: string }
	| { type: 'create_docx'; filename: string }
	| { type: 'mutate_docx_draft'; document_id: string }
	| { type: 'inspect_docx_draft'; document_id: string }
	| { type: 'finalize_docx'; document_id: string }
	| { type: 'create_pptx'; filename: string }
	| { type: 'mutate_pptx_draft'; deck_id: string }
	| { type: 'inspect_pptx_draft'; deck_id: string }
	| { type: 'finalize_pptx'; deck_id: string }
	| { type: 'create_pdf'; filename: string }
	| { type: 'mutate_pdf_draft'; pdf_id: string }
	| { type: 'inspect_pdf_draft'; pdf_id: string }
	| { type: 'finalize_pdf'; pdf_id: string };

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
	| { type: 'harness_state_updated'; sessionId: string; harnessState: AgentSessionHarnessState }
	| {
			type: 'session_paused';
			sessionId: string;
			message: string;
			pausedAt: string;
			pauseReason: PauseReason;
			currentSubtaskId?: string;
	  }
	| { type: 'recovery_continue'; sessionId: string }
	| { type: 'recovery_replan'; sessionId: string }
	| {
			type: 'clarification_replied';
			sessionId: string;
			content: string;
			subtaskId?: string;
	  }
	| {
			type: 'clarification_timed_out';
			sessionId: string;
			subtaskId?: string;
			prompt: string;
	  }
	| {
			type: 'clarification_restored';
			sessionId: string;
			prompt: string;
			askedAt: string;
			subtaskId?: string;
	  }
	| { type: 'execution_started'; sessionId: string; requestId: string }
	| {
			type: 'execution_resumed';
			sessionId: string;
			taskId: string;
			nextSubtaskId?: string;
	  }
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
			toolCallId: string;
			stepNumber: number;
			tool: string;
			arguments: Record<string, unknown>;
			content?: string;
	  }
	| {
			type: 'tool_result';
			sessionId: string;
			toolCallId: string;
			stepNumber: number;
			tool: string;
			result: Record<string, unknown>;
			isError: boolean;
			content?: string;
	  }
	| { type: 'task_created'; sessionId: string; task: BackendAgentTask }
	| {
			type: 'task_plan_generated';
			sessionId: string;
			taskId: string;
			title: string;
			description: string;
			isNonTrivial: boolean;
			subtasks: SubTask[];
	  }
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
			type: 'agent_narration';
			sessionId: string;
			conversationId: string;
			threadId: string;
			content: string;
			/**
			 * Narration intent (Workstream E.1). `question` signals that
			 * the backend paused the harness and is waiting for the user
			 * to reply; the AgentPanel renders those inline with a reply
			 * affordance. Defaults to `narration` when the field is
			 * missing on older events so pre-E payloads still parse.
			 */
			intent?: NarrationIntent;
			timestamp: string;
	  }
	| {
			type: 'streaming_content';
			sessionId: string;
			conversationId: string;
			threadId: string;
			chunk: string;
			isFinal: boolean;
			messageId?: string;
			persistMessage?: boolean;
	  }
	| {
			type: 'context_compacted';
			sessionId: string;
			conversationId: string;
			reason: 'token_limit' | 'subtask_transition';
			beforeCount: number;
			afterCount: number;
			beforeTokens: number;
			afterTokens: number;
			timestamp: string;
	  }
	| {
			type: 'pdf_phase_changed';
			sessionId: string;
			phase: string;
	  }
	| {
			type: 'pdf_progress';
			sessionId: string;
			percent: number;
	  }
	| {
			type: 'pdf_warning';
			sessionId: string;
			message: string;
	  }
	| {
			type: 'artifact_created';
			sessionId: string;
			artifactId: string;
			artifactType: string; // 'Docx' | 'Pptx' | 'Pdf'
			filename: string;
			sizeBytes: number;
			path: string;
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
	inputTokens?: number;
	outputTokens?: number;
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

/**
 * Check if a task is incomplete (still in progress or pending).
 */
export function isTaskIncomplete(task: AgentTask): boolean {
	return task.status === 'pending' || task.status === 'in_progress';
}

/**
 * Check if a session has any incomplete root tasks.
 */
export function hasIncompleteTasks(session: AgentSession): boolean {
	if (!session.tasks || session.tasks.length === 0) {
		return false;
	}
	// Only check root tasks (no parent), not subtasks
	return session.tasks.some((task) => task.subtasks === undefined && isTaskIncomplete(task));
}

/**
 * Get incomplete root task from a session, if any.
 */
export function getIncompleteTask(session: AgentSession): AgentTask | undefined {
	if (!session.tasks || session.tasks.length === 0) {
		return undefined;
	}
	return session.tasks.find((task) => task.subtasks === undefined && isTaskIncomplete(task));
}

/**
 * Check if subtasks have any incomplete work.
 */
export function hasIncompleteSubtasks(task: AgentTask): boolean {
	if (!task.subtasks || task.subtasks.length === 0) {
		return false;
	}
	return task.subtasks.some((st) => st.status === 'pending' || st.status === 'in_progress');
}

/**
 * Get a summary of completed vs incomplete subtasks.
 */
export function getSubtaskProgress(task: AgentTask): {
	total: number;
	completed: number;
	pending: number;
	inProgress: number;
	failed: number;
} {
	if (!task.subtasks || task.subtasks.length === 0) {
		return { total: 0, completed: 0, pending: 0, inProgress: 0, failed: 0 };
	}

	return {
		total: task.subtasks.length,
		completed: task.subtasks.filter((st) => st.status === 'completed').length,
		pending: task.subtasks.filter((st) => st.status === 'pending').length,
		inProgress: task.subtasks.filter((st) => st.status === 'in_progress').length,
		failed: task.subtasks.filter((st) => st.status === 'failed').length
	};
}
