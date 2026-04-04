/**
 * Global Popover Store
 * Ensures only one popover is open at a time using a custom event.
 */

const POPOVER_CLOSE_ALL_EVENT = 'popover-close-all';

/**
 * Request to open a popover with the given ID.
 * Dispatches an event to close all other currently open popovers.
 */
export function requestPopoverOpen(id: string): void {
	// Dispatch event to close all currently open popovers
	// The new popover will ignore this event since it's not open yet
	window.dispatchEvent(new CustomEvent(POPOVER_CLOSE_ALL_EVENT, { detail: { requestingId: id } }));
}

/**
 * Listen for close-all events.
 * Returns a cleanup function to remove the listener.
 * Callback receives the ID of the popover requesting to open,
 * so listeners can ignore the event if they are the requesting popover.
 */
export function onPopoverCloseAll(callback: (requestingId: string) => void): () => void {
	const handler = (e: Event) => {
		const event = e as CustomEvent<{ requestingId: string }>;
		callback(event.detail.requestingId);
	};
	window.addEventListener(POPOVER_CLOSE_ALL_EVENT, handler);
	return () => window.removeEventListener(POPOVER_CLOSE_ALL_EVENT, handler);
}
