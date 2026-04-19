/**
 * Tool-renderer contract.
 *
 * A `ToolRenderer` packages the three tool-specific decisions that used to
 * live as branches in `ToolCallMessage`'s switch statements: which icon to
 * show, how to build the header summary, and how to phrase the outcome.
 * Renderers can also opt into a custom body component (used by web_search
 * in Phase 9b to render its clickable result list).
 */
import type { Component } from 'svelte';

export type ToolCallStatus = 'pending' | 'success' | 'error';

export interface ToolSummary {
	title: string;
	subtitle?: string;
}

/**
 * Icon type that admits both Svelte 5 `Component`s and the legacy Svelte 4
 * component references exposed by `lucide-svelte`. The facade passes this
 * value to `<svelte:component this={icon} />` which handles both.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type IconComponent = Component<any, any, any> | any;

export interface ToolRenderer<TArgs = unknown, TResult = unknown> {
	/** Lucide (or compatible) component reference rendered in the header badge. */
	icon: IconComponent;
	/**
	 * Builds the header title + subtitle.
	 *
	 * @param args The tool-call arguments as produced by the backend. May be
	 *             `undefined` for in-flight calls that haven't emitted args
	 *             yet; renderers must tolerate this without throwing.
	 * @param toolName The full tool name the facade dispatched on. Used
	 *                 only by the default fallback to show the raw name for
	 *                 unknown tools; concrete renderers ignore it.
	 */
	getSummary(args: TArgs | undefined, toolName: string): ToolSummary;
	/**
	 * Builds the one-line outcome phrase rendered in the header (and as a
	 * fallback Result Overview line when no custom body is registered).
	 */
	getOutcome(result: TResult | undefined, status: ToolCallStatus): string;
	/**
	 * Optional custom body. When present, `ToolCallBody` renders this instead
	 * of the generic Result Overview block.
	 *
	 * Typed as `Component<any>` to allow body components to declare narrower
	 * `args`/`result` shapes without triggering contravariance errors. The
	 * caller (`ToolCallMessage`) always passes the full set of optional props:
	 *   `{ args, result, status, streamingContent?, touchedFile? }`
	 */
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	BodyComponent?: Component<any>;
}

/**
 * Shorthand for renderers that accept unknown `args` and `result` shapes.
 * Most renderers narrow these inline; the type alias exists so the
 * registry map can be typed without per-tool generics.
 */
export type AnyToolRenderer = ToolRenderer<unknown, unknown>;
