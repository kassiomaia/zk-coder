/**
 * Agent Status Utilities
 * Convert agent events and state into user-friendly status messages.
 */

import type {
	AgentEvent,
	AgentSessionHarnessState,
	ExecutionStep,
	SessionState,
	SubTask
} from '$lib/types/agent';

/** Current agent status with icon hint */
export interface AgentStatus {
	/** Status text to display */
	text: string;
	/** Icon hint for visual (e.g., 'scan', 'terminal', 'file', 'check') */
	icon: 'scan' | 'terminal' | 'file' | 'edit' | 'wait' | 'check' | 'sparkle';
	/** Whether the agent is currently active */
	active: boolean;
	/** Array of phrases to rotate through (from LLM-generated status updates) */
	phrases?: string[];
	/** Context usage percentage (0-100) - if > 50, will be shown in status */
	contextUsage?: number;
	/** Stall warning when LLM is taking too long (null = not stalled) */
	stallWarning?: 'still-working' | 'taking-longer' | 'very-long';
	/** Elapsed time since last activity in seconds (for display) */
	elapsedSeconds?: number;
}

/** Default idle status */
const IDLE_STATUS: AgentStatus = {
	text: 'Ready',
	icon: 'check',
	active: false
};

/** Default thinking status (when LLM is processing) */
const THINKING_STATUS: AgentStatus = {
	text: 'Analyzing...',
	icon: 'scan',
	active: true
};

/** Waiting for approval status */
const WAITING_STATUS: AgentStatus = {
	text: 'Waiting for approval...',
	icon: 'wait',
	active: true
};

/**
 * Convert session state to status message.
 */
export function stateToStatus(state: SessionState): AgentStatus {
	switch (state.type) {
		case 'launching':
			return { text: 'Starting...', icon: 'sparkle', active: true };
		case 'running':
			return { text: 'Thinking...', icon: 'sparkle', active: true };
		case 'wait_for_request':
			return { text: 'Ready', icon: 'check', active: false };
		case 'user_input_required':
			return { text: 'Waiting for your approval...', icon: 'wait', active: true };
		case 'paused':
			return { text: state.message || 'Paused', icon: 'wait', active: false };
		case 'finished':
			return { text: 'Task completed', icon: 'check', active: false };
		case 'cancelled':
			return { text: 'Task cancelled', icon: 'check', active: false };
		case 'error':
			return { text: 'Something went wrong', icon: 'check', active: false };
	}
}

function harnessToStatus(
	state: SessionState | null,
	harnessState: AgentSessionHarnessState | undefined
): AgentStatus | null {
	if (!harnessState) return null;

	const verification = harnessState.startupVerification;
	const environmentBlocked = harnessState.handoff?.environmentHealth === 'blocked';

	if (state?.type === 'running' && harnessState.mode === 'resume_verifier') {
		return {
			text: verification?.headline || 'Resuming from handoff...',
			icon: 'scan',
			active: true
		};
	}

	if (verification?.state === 'running') {
		return {
			text: verification.headline || 'Verifying environment...',
			icon: 'scan',
			active: true
		};
	}

	if (state?.type === 'paused' && verification?.state === 'failed') {
		return {
			text: verification.headline
				? `Resume blocked: ${verification.headline}`
				: 'Resume blocked',
			icon: 'wait',
			active: false
		};
	}

	if (verification?.state === 'failed') {
		return {
			text: verification.headline || 'Verification failed',
			icon: 'wait',
			active: false
		};
	}

	if (state?.type === 'paused' && environmentBlocked) {
		return {
			text: 'Environment blocked — investigate before resuming',
			icon: 'wait',
			active: false
		};
	}

	if (state?.type === 'paused' && harnessState.mode === 'resume_verifier') {
		return {
			text: verification?.headline || 'Ready to verify and resume',
			icon: 'wait',
			active: false
		};
	}

	if (state?.type === 'finished' && harnessState.cleanExitRecorded) {
		return {
			text: 'Clean handoff saved',
			icon: 'check',
			active: false
		};
	}

	return null;
}

/**
 * Extract file path from tool arguments.
 */
function extractFilePath(args: Record<string, unknown>): string | null {
	const path = (args.path as string | undefined) || (args.filePath as string | undefined);
	if (path) {
		// Get just the filename for brevity
		const parts = path.split(/[/\\]/);
		return parts[parts.length - 1] || path;
	}
	return null;
}

/**
 * Extract command from tool arguments.
 */
function extractCommand(args: Record<string, unknown>): string | null {
	const cmd = (args.command as string | undefined) || (args.cmd as string | undefined);
	if (cmd) {
		// Truncate long commands
		return cmd.length > 30 ? cmd.slice(0, 27) + '...' : cmd;
	}
	return null;
}

/**
 * Convert execution step to status message.
 */
export function stepToStatus(step: ExecutionStep): AgentStatus | null {
	const { action } = step;

	switch (action.type) {
		case 'llm_request':
			return { text: 'Planning next steps...', icon: 'sparkle', active: true };

		case 'tool_call': {
			const tool = action.tool;

			switch (tool) {
				case 'read_file':
				case 'read': {
					const path = extractFilePath(action.arguments);
					return path
						? { text: `Reading ${path}`, icon: 'file', active: true }
						: { text: 'Reading project files...', icon: 'file', active: true };
				}
				case 'write_file':
				case 'write':
				case 'edit_file':
				case 'edit': {
					const path = extractFilePath(action.arguments);
					return path
						? { text: `Updating ${path}`, icon: 'edit', active: true }
						: { text: 'Applying changes...', icon: 'edit', active: true };
				}
				case 'execute_command':
				case 'bash': {
					const cmd = extractCommand(action.arguments);
					return cmd
						? { text: `Executing: ${cmd}`, icon: 'terminal', active: true }
						: { text: 'Running system command...', icon: 'terminal', active: true };
				}
				case 'grep':
				case 'glob': {
					return { text: 'Searching codebase...', icon: 'scan', active: true };
				}
				default:
					return { text: `Using ${tool}...`, icon: 'sparkle', active: true };
			}
		}
	}

	return null;
}

/**
 * Convert subtask to status message.
 */
export function subtaskToStatus(subtask: SubTask): AgentStatus | null {
	if (subtask.status === 'completed') return null;

	// Use subtask title, truncate if too long
	const title = subtask.title.length > 25 ? subtask.title.slice(0, 22) + '...' : subtask.title;
	return {
		text: title,
		icon: 'sparkle',
		active: subtask.status === 'in_progress'
	};
}

/**
 * Get current status from agent events and state.
 * Priority: approval pending > LLM status phrases > latest step > subtask > state > default
 */
export function getAgentStatus(
	state: SessionState | null,
	latestStep: ExecutionStep | null,
	currentSubtask: SubTask | null,
	hasPendingApproval: boolean,
	statusPhrases?: string[],
	contextUsage?: number,
	lastActivityAt?: Date | null,
	harnessState?: AgentSessionHarnessState
): AgentStatus {
	// Extract context usage from state if not provided
	const usage = contextUsage ?? (state?.type === 'running' ? state.contextUsage : 0);

	// Compute stall warning based on last activity time
	let stallWarning: AgentStatus['stallWarning'] = undefined;
	let elapsedSeconds = 0;

	if (lastActivityAt && state?.type === 'running') {
		const now = new Date();
		const elapsedMs = now.getTime() - lastActivityAt.getTime();
		elapsedSeconds = Math.floor(elapsedMs / 1000);

		if (elapsedSeconds >= 120) {
			stallWarning = 'very-long'; // 2+ minutes
		} else if (elapsedSeconds >= 60) {
			stallWarning = 'taking-longer'; // 1+ minute
		} else if (elapsedSeconds >= 30) {
			stallWarning = 'still-working'; // 30+ seconds
		}
	}

	// Helper to add stall info to status
	const addStallInfo = (status: AgentStatus): AgentStatus => {
		if (!stallWarning || !status.active) return status;
		return {
			...status,
			stallWarning,
			elapsedSeconds
		};
	};

	// Approval pending takes priority
	if (hasPendingApproval) {
		return WAITING_STATUS;
	}

	const harnessStatus = harnessToStatus(state, harnessState);
	if (harnessStatus) {
		if (usage > 50 && harnessStatus.active) {
			return addStallInfo({
				...harnessStatus,
				text: `${harnessStatus.text} (${Math.round(usage)}%)`,
				contextUsage: usage
			});
		}
		return addStallInfo(harnessStatus);
	}

	// Once the session is no longer actively running, ignore stale execution steps and phrases.
	// Otherwise old thread sessions can look "in progress" forever based on historical telemetry.
	if (state && state.type !== 'running') {
		const status = stateToStatus(state);
		if (usage > 50) {
			return addStallInfo({
				...status,
				text: `${status.text} (${Math.round(usage)}%)`,
				contextUsage: usage
			});
		}
		return addStallInfo(status);
	}

	// LLM-generated status phrases take priority when available and agent is running
	if (state?.type === 'running' && statusPhrases && statusPhrases.length > 0) {
		let text = statusPhrases[statusPhrases.length - 1];
		// Append context usage if > 50%
		if (usage > 50) {
			text += ` (${Math.round(usage)}%)`;
		}
		return addStallInfo({
			text,
			icon: 'sparkle',
			active: true,
			phrases: statusPhrases,
			contextUsage: usage
		});
	}

	// Latest execution step
	if (latestStep) {
		const stepStatus = stepToStatus(latestStep);
		if (stepStatus) {
			// Append context usage if > 50%
			if (usage > 50) {
				return addStallInfo({
					...stepStatus,
					text: `${stepStatus.text} (${Math.round(usage)}%)`,
					contextUsage: usage
				});
			}
			return addStallInfo(stepStatus);
		}
	}

	// Current in-progress subtask
	if (currentSubtask && currentSubtask.status === 'in_progress') {
		const subtaskStatus = subtaskToStatus(currentSubtask);
		if (subtaskStatus) {
			// Append context usage if > 50%
			if (usage > 50) {
				return addStallInfo({
					...subtaskStatus,
					text: `${subtaskStatus.text} (${Math.round(usage)}%)`,
					contextUsage: usage
				});
			}
			return addStallInfo(subtaskStatus);
		}
	}

	// Fall back to session state
	if (state) {
		const status = stateToStatus(state);
		// Append context usage if > 50%
		if (usage > 50) {
			return addStallInfo({
				...status,
				text: `${status.text} (${Math.round(usage)}%)`,
				contextUsage: usage
			});
		}
		return addStallInfo(status);
	}

	// Default idle
	return IDLE_STATUS;
}

/**
 * Track agent status from a stream of events.
 * Call this with each new event to get the updated status.
 */
export function trackAgentStatus(events: AgentEvent[]): AgentStatus {
	let state: SessionState | null = null;
	let latestStep: ExecutionStep | null = null;
	let currentSubtask: SubTask | null = null;
	let hasPendingApproval = false;

	// Process events in order (most recent at end)
	for (const event of events) {
		switch (event.type) {
			case 'session_state_changed':
				state = event.state;
				break;
			case 'execution_step':
				latestStep = event.step;
				break;
			case 'subtask_created':
			case 'subtask_updated':
				if (event.subtask.status === 'in_progress') {
					currentSubtask = event.subtask;
				}
				break;
			case 'approval_required':
				hasPendingApproval = true;
				break;
			case 'approval_approved':
			case 'approval_denied':
				hasPendingApproval = false;
				break;
		}
	}

	return getAgentStatus(state, latestStep, currentSubtask, hasPendingApproval);
}
