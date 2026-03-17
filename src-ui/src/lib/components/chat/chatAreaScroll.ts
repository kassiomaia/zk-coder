export type MessagesLoadingState = 'idle' | 'loading' | 'success' | 'error';

interface InitialBottomScrollArgs {
	currentThreadId?: string;
	pendingThreadId: string | null;
	messagesLoading: MessagesLoadingState;
}

/**
 * Thread open should only force bottom when the selected thread has fully loaded.
 * This guard prevents repeated forcing during normal scrolling after initial open.
 */
export function shouldForceInitialBottomScroll({
	currentThreadId,
	pendingThreadId,
	messagesLoading
}: InitialBottomScrollArgs): boolean {
	if (!currentThreadId) return false;
	if (pendingThreadId !== currentThreadId) return false;
	return messagesLoading === 'success';
}
