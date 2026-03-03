/**
 * Intentional Delay Utility
 *
 * Provides a cancellable delay mechanism for destructive actions,
 * giving users time to reconsider before the action executes.
 */

export interface IntentionalDelayOptions {
	/** Delay in milliseconds before action executes */
	delay?: number;
	/** Callback when delay starts */
	onStart?: () => void;
	/** Callback when delay is cancelled (user moved away) */
	onCancel?: () => void;
	/** Callback when delay completes */
	onComplete?: () => void;
}

export interface IntentionalDelayReturn {
	/** Start the delay timer */
	start: () => void;
	/** Cancel the delay timer */
	cancel: () => void;
	/** Whether the delay is currently active */
	isActive: boolean;
	/** Whether the delay has completed */
	isCompleted: boolean;
}

/**
 * Creates an intentional delay for confirmatory actions.
 *
 * @example
 * ```svelte
 * <script>
 *   const deleteDelay = useIntentionalDelay({
 *     delay: 300,
 *     onComplete: () => deleteItem()
 *   });
 * </script>
 *
 * <button
 *   onmousedown={deleteDelay.start}
 *   onmouseleave={deleteDelay.cancel}
 *   onmouseup={deleteDelay.cancel}
 * >
 *   Delete (hold)
 * </button>
 * ```
 */
export function useIntentionalDelay(options: IntentionalDelayOptions = {}): IntentionalDelayReturn {
	const { delay = 300, onStart, onCancel, onComplete } = options;

	let timeoutId: ReturnType<typeof setTimeout> | null = null;
	let isActive = $state(false);
	let isCompleted = $state(false);

	function start() {
		if (isActive) return;

		isActive = true;
		isCompleted = false;
		onStart?.();

		timeoutId = setTimeout(() => {
			isActive = false;
			isCompleted = true;
			onComplete?.();
		}, delay);
	}

	function cancel() {
		if (timeoutId) {
			clearTimeout(timeoutId);
			timeoutId = null;
		}

		if (isActive) {
			isActive = false;
			onCancel?.();
		}
	}

	return {
		start,
		cancel,
		get isActive() {
			return isActive;
		},
		get isCompleted() {
			return isCompleted;
		}
	};
}

/**
 * Creates a promise-based intentional delay.
 * Useful for async handlers.
 *
 * @example
 * ```ts
 * async function handleDelete() {
 *   await intentionalDelayPromise(300);
 *   await deleteItem();
 * }
 * ```
 */
export function intentionalDelayPromise(delay = 300): Promise<void> {
	return new Promise((resolve) => {
		setTimeout(resolve, delay);
	});
}

/**
 * Creates a cancellable promise-based delay.
 * Returns an object with the promise and a cancel function.
 *
 * @example
 * ```ts
 * const { promise, cancel } = createCancellableDelay(300);
 *
 * element.addEventListener('mouseleave', cancel);
 *
 * await promise;
 * deleteItem();
 * ```
 */
export function createCancellableDelay(delay = 300): {
	promise: Promise<boolean>;
	cancel: () => void;
} {
	let timeoutId: ReturnType<typeof setTimeout>;
	let cancelled = false;

	const promise = new Promise<boolean>((resolve) => {
		timeoutId = setTimeout(() => {
			resolve(!cancelled);
		}, delay);
	});

	function cancel() {
		cancelled = true;
		clearTimeout(timeoutId);
	}

	return { promise, cancel };
}
