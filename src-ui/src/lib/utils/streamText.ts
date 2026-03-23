/**
 * Simulate streaming text effect by updating a message incrementally.
 *
 * @param messageId - ID of the message to update
 * @param fullContent - Complete content to stream
 * @param updateFn - Function to call with each content update
 * @param options - Stream options
 * @returns Cleanup function to cancel streaming
 */
export function streamText(
	messageId: string,
	fullContent: string,
	updateFn: (content: string) => void,
	options: {
		/** Characters per batch (default: 3) */
		charBatchSize?: number;
		/** Delay between batches in ms (default: 10ms) */
		batchDelay?: number;
		/** Initial delay before starting (default: 50ms) */
		initialDelay?: number;
	} = {}
): () => void {
	const {
		charBatchSize = 3,
		batchDelay = 10,
		initialDelay = 50
	} = options;

	let cancelled = false;
	let timeoutId: ReturnType<typeof setTimeout> | null = null;
	let currentIndex = 0;

	const streamNext = () => {
		if (cancelled) return;

		if (currentIndex >= fullContent.length) {
			// Ensure final content is set exactly
			updateFn(fullContent);
			return;
		}

		// Calculate next chunk
		const nextIndex = Math.min(currentIndex + charBatchSize, fullContent.length);
		const partialContent = fullContent.slice(0, nextIndex);
		currentIndex = nextIndex;

		updateFn(partialContent);

		// Schedule next batch
		timeoutId = setTimeout(streamNext, batchDelay);
	};

	// Start streaming after initial delay
	timeoutId = setTimeout(streamNext, initialDelay);

	// Return cleanup function
	return () => {
		cancelled = true;
		if (timeoutId) {
			clearTimeout(timeoutId);
			timeoutId = null;
		}
	};
}

/**
 * Stream text markdown-aware - try to break at word boundaries and markdown blocks.
 */
export function streamMarkdown(
	messageId: string,
	fullContent: string,
	updateFn: (content: string) => void,
	options: {
		/** Chunks per batch (default: 2-4 characters for smooth typing effect) */
		charBatchSize?: number;
		/** Delay between batches in ms (default: 8ms for faster feel) */
		batchDelay?: number;
		/** Initial delay before starting (default: 30ms) */
		initialDelay?: number;
	} = {}
): () => void {
	// Use same implementation for now - could be enhanced for markdown awareness
	return streamText(messageId, fullContent, updateFn, options);
}
