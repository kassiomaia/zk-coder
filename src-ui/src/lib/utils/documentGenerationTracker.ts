/**
 * Document Generation Tracker
 *
 * Aggregates tool calls related to document generation into a unified state.
 * Detects document generation workflows (DOCX, PDF, PPTX) and maps tool calls
 * to human-readable progress phases.
 *
 * Tool patterns detected:
 * - create_docx, create_pptx, create_pdf -> Start generation
 * - mutate_*_draft, add_section, add_slide -> Building content
 * - finalize_* -> Finalizing document
 */

import type { Message } from '$lib/types/message';
import type {
	DocType,
	GenPhase,
	ProgressStep
} from '$lib/components/chat/messages/DocumentGenerationCard.svelte';

/** Document generation job state */
export interface DocumentGenerationJob {
	/** Unique ID for this job (session-based) */
	id: string;
	/** Session ID this job belongs to */
	sessionId: string;
	/** Document type */
	docType: DocType;
	/** Filename of the document */
	filename: string;
	/** Current phase */
	phase: GenPhase;
	/** Human-readable status text */
	statusText: string;
	/** Progress percentage (0-100 or null) */
	progress: number | null;
	/** Individual progress steps */
	steps: ProgressStep[];
	/** Error message if failed */
	error?: string;
	/** Whether this job is complete */
	complete: boolean;
	/** Artifact ID when complete */
	artifactId?: string;
	/** File path when complete */
	filePath?: string;
	/** Timestamps */
	startedAt: Date;
	updatedAt: Date;
}

/** Tool name patterns for document generation */
const DOC_START_TOOLS = ['create_docx', 'create_pptx', 'create_pdf'];
const DOC_MUTATE_TOOLS = [
	'mutate_docx_draft',
	'mutate_pptx_draft',
	'mutate_pdf_draft',
	'add_section',
	'add_slide',
	'add_page',
	'write_section',
	'write_slide',
	'insert_content'
];
const DOC_INSPECT_TOOLS = ['inspect_docx_draft', 'inspect_pptx_draft', 'inspect_pdf_draft'];
const DOC_FINALIZE_TOOLS = ['finalize_docx', 'finalize_pptx', 'finalize_pdf'];

/** Extract document type from tool name */
function extractDocType(tool: string): DocType | null {
	if (tool.includes('docx')) return 'Docx';
	if (tool.includes('pptx')) return 'Pptx';
	if (tool.includes('pdf')) return 'Pdf';
	return null;
}

/** Map tool to document type */
function getDocTypeFromTool(tool: string): DocType | null {
	const lower = tool.toLowerCase();
	if (lower.includes('docx') || lower.includes('document') || lower.includes('.doc')) {
		return 'Docx';
	}
	if (lower.includes('pptx') || lower.includes('presentation') || lower.includes('slide')) {
		return 'Pptx';
	}
	if (lower.includes('pdf')) {
		return 'Pdf';
	}
	return null;
}

/** Check if a tool is related to document generation */
function isDocumentGenerationTool(tool: string): boolean {
	const lower = tool.toLowerCase();
	return [
		...DOC_START_TOOLS,
		...DOC_MUTATE_TOOLS,
		...DOC_INSPECT_TOOLS,
		...DOC_FINALIZE_TOOLS,
		'start_document',
		'build_document',
		'generate_document',
		'create_document',
		'write_document'
	].some((t) => lower.includes(t) || t.includes(lower));
}

/** Extract filename from tool arguments */
function extractFilename(args: Record<string, unknown>): string {
	if (typeof args.filename === 'string') return args.filename;
	if (typeof args.name === 'string') return args.name;
	if (typeof args.path === 'string') {
		const parts = args.path.split(/[/\\]/);
		return parts[parts.length - 1] || 'Untitled';
	}
	return 'Untitled Document';
}

/** Map tool name to human-readable phase */
function getPhaseForTool(tool: string): { phase: GenPhase; text: string } {
	const lower = tool.toLowerCase();

	// Start tools
	if (DOC_START_TOOLS.some((t) => lower.includes(t))) {
		return { phase: 'preparing', text: 'Preparing document...' };
	}

	// Finalize tools
	if (DOC_FINALIZE_TOOLS.some((t) => lower.includes(t))) {
		return { phase: 'finalizing', text: 'Finalizing document...' };
	}

	// Default to building
	if (DOC_MUTATE_TOOLS.some((t) => lower.includes(t))) {
		const docType = getDocTypeFromTool(tool);
		if (docType === 'Pptx' || lower.includes('slide')) {
			return { phase: 'building', text: 'Building slides...' };
		}
		return { phase: 'building', text: 'Writing sections...' };
	}

	return { phase: 'building', text: 'Processing...' };
}

/** Generate default steps based on document type */
function generateDefaultSteps(docType: DocType): ProgressStep[] {
	switch (docType) {
		case 'Docx':
			return [
				{ label: 'Preparing content', completed: false, current: true },
				{ label: 'Building sections', completed: false },
				{ label: 'Formatting document', completed: false },
				{ label: 'Finalizing', completed: false }
			];
		case 'Pptx':
			return [
				{ label: 'Preparing presentation', completed: false, current: true },
				{ label: 'Creating slides', completed: false },
				{ label: 'Applying formatting', completed: false },
				{ label: 'Finalizing', completed: false }
			];
		case 'Pdf':
			return [
				{ label: 'Preparing content', completed: false, current: true },
				{ label: 'Building pages', completed: false },
				{ label: 'Generating PDF', completed: false },
				{ label: 'Finalizing', completed: false }
			];
	}
}

/** Calculate progress from phase and steps */
function calculateProgress(phase: GenPhase, steps: ProgressStep[]): number | null {
	const totalSteps = steps.length;
	const completedSteps = steps.filter((s) => s.completed).length;
	const currentStepIndex = steps.findIndex((s) => s.current);

	if (totalSteps === 0) return null;

	// Base progress from completed steps
	let baseProgress = (completedSteps / totalSteps) * 100;

	// Add partial progress for current step
	if (currentStepIndex >= 0 && !steps[currentStepIndex].completed) {
		// Add 50% of the current step's contribution
		const stepProgress = (1 / totalSteps) * 50;
		baseProgress += stepProgress;
	}

	// Phase-based adjustments
	if (phase === 'preparing') {
		return Math.min(baseProgress, 15);
	}
	if (phase === 'finalizing') {
		return Math.max(baseProgress, 85);
	}

	return Math.min(Math.round(baseProgress), 100);
}

/** Document generation tracker class */
export class DocumentGenerationTracker {
	private jobs = new Map<string, DocumentGenerationJob>();

	/** Get all active jobs */
	getActiveJobs(): DocumentGenerationJob[] {
		return Array.from(this.jobs.values()).filter((j) => !j.complete);
	}

	/** Get job by ID */
	getJob(id: string): DocumentGenerationJob | undefined {
		return this.jobs.get(id);
	}

	/** Get job by session ID */
	getJobBySession(sessionId: string): DocumentGenerationJob | undefined {
		return Array.from(this.jobs.values()).find((j) => j.sessionId === sessionId && !j.complete);
	}

	/** Process a message and update jobs if relevant */
	processMessage(message: Message): DocumentGenerationJob | null {
		// Only process tool_call messages
		if (message.kind !== 'tool_call') return null;

		const toolName = message.agentData?.toolName;
		if (!toolName || !isDocumentGenerationTool(toolName)) return null;

		const sessionId = message.agentData?.sessionId || message.threadId;
		const args = message.agentData?.toolArgs || {};
		const result = message.agentData?.toolResult;
		const isError = message.agentData?.toolError;
		const status = message.agentData?.toolCallStatus;

		// Get or create job
		let job = this.getJobBySession(sessionId);

		if (!job) {
			// Check if this is a start tool
			const docType = getDocTypeFromTool(toolName);
			if (!docType) return null;

			const filename = extractFilename(args);
			const jobId = `${sessionId}-${docType}-${Date.now()}`;

			job = {
				id: jobId,
				sessionId,
				docType,
				filename,
				phase: 'preparing',
				statusText: 'Starting document generation...',
				progress: 0,
				steps: generateDefaultSteps(docType),
				complete: false,
				startedAt: message.timestamp,
				updatedAt: message.timestamp
			};

			this.jobs.set(jobId, job);
		}

		// Update job based on tool
		this.updateJobFromTool(job, toolName, args, result, isError, status);

		return job;
	}

	/** Update job from tool call */
	private updateJobFromTool(
		job: DocumentGenerationJob,
		toolName: string,
		args: Record<string, unknown>,
		result: unknown,
		isError: boolean | undefined,
		status: string | undefined
	): void {
		const phaseInfo = getPhaseForTool(toolName);

		// Handle error
		if (isError || status === 'error') {
			job.phase = 'failed';
			job.statusText = 'Generation failed';
			job.error = this.extractErrorMessage(result);
			job.updatedAt = new Date();
			return;
		}

		// Handle completion from finalize
		if (DOC_FINALIZE_TOOLS.some((t) => toolName.toLowerCase().includes(t))) {
			if (status === 'success' || result) {
				job.phase = 'completed';
				job.statusText = 'Document ready';
				job.progress = 100;
				job.complete = true;
				job.steps.forEach((s) => {
					s.completed = true;
					s.current = false;
				});

				// Extract artifact info from result
				if (typeof result === 'object' && result !== null) {
					const res = result as Record<string, unknown>;
					if (typeof res.artifact_id === 'string') {
						job.artifactId = res.artifact_id;
					}
					if (typeof res.path === 'string') {
						job.filePath = res.path;
					}
				}

				job.updatedAt = new Date();
				return;
			}
		}

		// Update phase and status
		job.phase = phaseInfo.phase;
		job.statusText = phaseInfo.text;

		// Update steps based on phase
		this.updateStepsForPhase(job, phaseInfo.phase);

		// Calculate progress
		job.progress = calculateProgress(job.phase, job.steps);

		job.updatedAt = new Date();
	}

	/** Update progress steps based on current phase */
	private updateStepsForPhase(job: DocumentGenerationJob, phase: GenPhase): void {
		// Mark previous steps as complete, find current step
		let foundCurrent = false;

		for (const step of job.steps) {
			if (foundCurrent) {
				// Future steps: reset
				step.completed = false;
				step.current = false;
				continue;
			}

			if (step.current) {
				// This was the current step
				if (this.shouldCompleteStep(step.label, phase)) {
					step.completed = true;
					step.current = false;
				}
				foundCurrent = !step.completed;
			} else if (!step.completed) {
				// First incomplete step becomes current
				if (!foundCurrent) {
					step.current = true;
					foundCurrent = true;
				}
			}
		}
	}

	/** Determine if a step should be marked complete for a phase */
	private shouldCompleteStep(stepLabel: string, phase: GenPhase): boolean {
		switch (phase) {
			case 'preparing':
				return false;
			case 'building':
				return stepLabel.toLowerCase().includes('prepar');
			case 'finalizing':
				return (
					stepLabel.toLowerCase().includes('prepar') ||
					stepLabel.toLowerCase().includes('build') ||
					stepLabel.toLowerCase().includes('writ') ||
					stepLabel.toLowerCase().includes('creat')
				);
			default:
				return false;
		}
	}

	/** Extract error message from result */
	private extractErrorMessage(result: unknown): string {
		if (typeof result === 'string') return result;
		if (typeof result === 'object' && result !== null) {
			const res = result as Record<string, unknown>;
			if (typeof res.error === 'string') return res.error;
			if (typeof res.message === 'string') return res.message;
		}
		return 'An unknown error occurred';
	}

	/** Mark job as retrying */
	retryJob(jobId: string): boolean {
		const job = this.jobs.get(jobId);
		if (!job) return false;

		job.phase = 'preparing';
		job.statusText = 'Retrying...';
		job.error = undefined;
		job.progress = 0;

		// Reset steps
		job.steps.forEach((s, i) => {
			s.completed = false;
			s.current = i === 0;
		});

		job.updatedAt = new Date();
		return true;
	}

	/** Remove a job */
	removeJob(jobId: string): boolean {
		return this.jobs.delete(jobId);
	}

	/** Clear completed jobs older than specified milliseconds */
	clearOldJobs(maxAgeMs: number = 5 * 60 * 1000): void {
		const now = Date.now();
		for (const [id, job] of this.jobs.entries()) {
			if (job.complete && now - job.updatedAt.getTime() > maxAgeMs) {
				this.jobs.delete(id);
			}
		}
	}
}

/** Global tracker instance */
export const documentGenerationTracker = new DocumentGenerationTracker();

/** Utility function to check if a message should be hidden (superseded by doc gen card) */
export function shouldHideMessageInDocGen(message: Message): boolean {
	if (message.kind !== 'tool_call') return false;

	const toolName = message.agentData?.toolName?.toLowerCase();
	if (!toolName) return false;

	// Check if there's an active doc gen job for this session
	const sessionId = message.agentData?.sessionId || message.threadId;
	const job = documentGenerationTracker.getJobBySession(sessionId);

	if (!job) return false;

	// Hide if it's a document generation tool and job is active
	return isDocumentGenerationTool(toolName);
}

/** Get document generation state for a session */
export function getDocumentGenerationState(sessionId: string): DocumentGenerationJob | null {
	return documentGenerationTracker.getJobBySession(sessionId) || null;
}
