<script lang="ts">
	/**
	 * ChatArea Component (Enhanced)
	 * Main chat interface with message display and input.
	 */
	import ChatInput from './input/ChatInput.svelte';
	import AnsweringIndicator from './AnsweringIndicator.svelte';
	import ThreadIndicator from './ThreadIndicator.svelte';
	import MarkdownRenderer from '../markdown/MarkdownRenderer.svelte';
	import MessageActions from './MessageActions.svelte';
	import type { Message } from '$lib/types/message';
	import type { Thread } from '$lib/types/thread';
	import styles from './ChatArea.module.css';
	import { isLlmStreaming, streamingContent } from '$lib/stores/chat';
	import { notifyAssistantComplete, initNotifications } from '$lib/utils/notifications';
	import {
		saveScrollPosition,
        restoreScrollPosition,
        hasSavedPosition,
        initializeThreadScroll,
        cleanupThreadScroll
    } from '$lib/utils/scrollManager';
    import {
        registerThreadElement,
        unregisterThread,
        handleStreamingStart,
        handleStreamingEnd,
        handleContentUpdate,
        handleUserScroll as controllerHandleUserScroll,
        scrollToBottom as controllerScrollToBottom,
        enableAutoScroll as controllerEnableAutoScroll,
        isAutoScrollEnabled as controllerIsAutoScrollEnabled
    } from '$lib/utils/scrollController';
    import { onMount } from 'svelte';

    onMount(() => {
        initNotifications();
    });

    interface Props {
        /** Array of messages to display */
        messages: Message[];

        /** Current thread ID for scroll position tracking */
        threadId?: string;

        /** Available threads for looking up thread info */
        threads?: Thread[];

        /** Loading state */
        loading?: boolean;

        /** Is assistant typing */
        isTyping?: boolean;

        /** Error message to display */
        error?: string;

        /** Loading state for messages */
        messagesLoading?: 'idle' | 'loading' | 'success' | 'error';

        /** Thread is resolved - disables input */
        threadResolved?: boolean;

        /** Send message handler */
        onSendMessage?: (content: string) => void;

        /** Retry handler */
        onRetry?: () => void;

        /** Edit message handler */
        onEditMessage?: (id: string) => void;

        /** Delete message handler */
        onDeleteMessage?: (id: string) => void;

        /** Retry message handler (for failed messages) */
        onRetryMessage?: (id: string) => void;

        /** Thread click handler */
        onThreadClick?: (threadId: string) => void;
    }

    let {
        messages,
        threadId,
        threads = [],
        loading = false,
        isTyping = false,
        error,
        messagesLoading = 'idle',
        threadResolved = false,
        onSendMessage,
        onRetry,
        onEditMessage,
        onDeleteMessage,
        onRetryMessage,
        onThreadClick
    }: Props = $props();

    // Input state
    let inputValue = $state('');

    // Animation state for input container
    let inputAnimatingIn = $state(false);
    let inputAnimatingOut = $state(false);
    let showInput = $state(true);
    let previousResolved = $state(false);

    // Scroll state
    let messagesWrapperRef: HTMLDivElement | undefined = $state();
    let previousThreadId = $state<string | undefined>(undefined);
    let isUserScrolling = $state(false);

    // Track if we should show "scroll to bottom" button
    let showScrollButton = $state(false);

    // Track previous streaming state to detect completion
    let wasStreaming = $state(false);

    // Track last scroll position to detect user scrolling UP
    let lastScrollTop = $state(0);

    // Threshold for showing scroll button (larger zone for visibility)
    const SCROLL_BUTTON_THRESHOLD = 100;

    // Watch for streaming state changes using the new controller
    $effect(() => {
        const streaming = $isLlmStreaming;
        const currentThreadId = threadId;
        const ref = messagesWrapperRef;

        console.log('[ChatArea] streaming state effect:', {
            streaming,
            wasStreaming,
            currentThreadId,
            hasRef: !!ref
        });

        if (!ref || !currentThreadId) return;

        // Detect transition from NOT streaming TO streaming
        if (!wasStreaming && streaming) {
            // Streaming just started - notify controller
            console.log('[ChatArea] streaming START detected');
            handleStreamingStart(currentThreadId);
        }

        // Detect transition from streaming to NOT streaming (completion)
        if (wasStreaming && !streaming) {
            // Streaming just completed - notify controller
            console.log('[ChatArea] streaming END detected');
            handleStreamingEnd(currentThreadId);

            // Check if user was scrolled up (auto-scroll was disabled)
            const autoScrollWasEnabled = controllerIsAutoScrollEnabled(currentThreadId);

            if (!autoScrollWasEnabled) {
                // User was scrolled up - send notification
                notifyAssistantComplete(
                    'Assistant Complete',
                    'The assistant has finished responded. Click to see the response.'
                );
            }
        }

        wasStreaming = streaming;
    });

    // Watch for threadResolved changes and animate transitions
    $effect(() => {
        const currentResolved = threadResolved;

        // First run - initialize state without animation
        if (previousResolved === false && !inputAnimatingIn && !inputAnimatingOut) {
            showInput = !currentResolved;
            previousResolved = currentResolved;
            return;
        }

        // Only animate when the resolved state actually changes
        if (currentResolved !== previousResolved) {
            const wasResolved = previousResolved;
            const nowResolved = currentResolved;

            previousResolved = currentResolved;

            if (wasResolved && !nowResolved) {
                // Transition: locked -> unlocked
                // Wait for banner to slide out (200ms), then slide input up
                setTimeout(() => {
                    showInput = true;
                    inputAnimatingIn = true;
                    setTimeout(() => {
                        inputAnimatingIn = false;
                    }, 200);
                }, 200);
            } else if (!wasResolved && nowResolved) {
                // Transition: unlocked -> locked
                // Animate input out (slide down), then banner will slide in
                inputAnimatingOut = true;
                setTimeout(() => {
                    showInput = false;
                    inputAnimatingOut = false;
                }, 150);
            }
            // If wasResolved and nowResolved: both locked - no animation (stay hidden)
            // If !wasResolved && !nowResolved: both unlocked - no animation (stay visible)
        }
    });

    // Handle thread switching - register/unregister with controller
    $effect(() => {
        const currentThreadId = threadId;
        const ref = messagesWrapperRef;

        console.log('[ChatArea] thread registration effect:', {
            currentThreadId,
            previousThreadId,
            hasRef: !!ref
        });

        // Save position for previous thread
        if (previousThreadId && ref) {
            saveScrollPosition(previousThreadId, ref);
        }

        // Unregister previous thread from controller
        if (previousThreadId && previousThreadId !== currentThreadId) {
            unregisterThread(previousThreadId);
            cleanupThreadScroll(previousThreadId);
            // Reset scroll tracking for new thread
            lastScrollTop = 0;
        }

        // Register new thread with controller
        if (ref && currentThreadId && currentThreadId !== previousThreadId) {
            console.log('[ChatArea] registering thread:', currentThreadId);
            registerThreadElement(currentThreadId, ref);
            initializeThreadScroll(currentThreadId);
        }

        previousThreadId = currentThreadId;

        // Restore position or scroll to bottom
        if (ref && currentThreadId) {
            if (hasSavedPosition(currentThreadId)) {
                restoreScrollPosition(currentThreadId, ref);
            } else {
                // New thread - scroll to bottom immediately (no animation, no RAF)
                // This prevents the visual "flash" of seeing top before scrolling
                ref.scrollTop = ref.scrollHeight;
            }
            // Update lastScrollTop after setting position
            lastScrollTop = ref.scrollTop;
        }
    });

    // Auto-scroll when messages change - using new controller
    $effect(() => {
        const ref = messagesWrapperRef;
        const currentThreadId = threadId;

        // Trigger on messages length change
        void messages.length;

        // Also trigger on streaming content changes to scroll during streaming
        // This ensures we scroll when streaming content is being added to existing messages
        void $streamingContent.length;

        console.log('[ChatArea] content update effect:', {
            hasRef: !!ref,
            currentThreadId,
            messagesLength: messages.length,
            streamingContentLength: $streamingContent.length
        });

        if (!ref || !currentThreadId) return;

        // Notify controller of content update
        handleContentUpdate(currentThreadId);

        // Show button when outside the scroll button threshold zone
        const distanceFromBottom = ref.scrollHeight - ref.scrollTop - ref.clientHeight;
        showScrollButton = distanceFromBottom > SCROLL_BUTTON_THRESHOLD;
    });

    // Handle scroll events - using new controller
    function handleScroll() {
        const ref = messagesWrapperRef;
        const currentThreadId = threadId;
        if (!ref || !currentThreadId) return;

        const currentScrollTop = ref.scrollTop;
        const scrolledUp = currentScrollTop < lastScrollTop - 5; // 5px threshold to ignore minor changes
        lastScrollTop = currentScrollTop;

        // Notify controller of user scroll
        controllerHandleUserScroll(currentThreadId);
    }

    // Manual scroll to bottom (when user clicks the button)
    function handleScrollToBottom() {
        const ref = messagesWrapperRef;
        const currentThreadId = threadId;
        if (!ref || !currentThreadId) return;

        controllerEnableAutoScroll(currentThreadId);
        controllerScrollToBottom(currentThreadId, { priority: 'user', animated: true });
        showScrollButton = false;
        isUserScrolling = false;
        lastScrollTop = ref.scrollTop;
    }

    // Re-enable auto-scroll and scroll to bottom when user sends a message
    function handleSubmit() {
        const ref = messagesWrapperRef;
        const currentThreadId = threadId;

        if (currentThreadId) {
            controllerEnableAutoScroll(currentThreadId);
        }

        if (inputValue.trim() && onSendMessage) {
            onSendMessage(inputValue.trim());
            inputValue = '';
        }

        // Delay scroll slightly to ensure user's message appears in DOM first
        if (ref && currentThreadId) {
            setTimeout(() => {
                controllerScrollToBottom(currentThreadId, { priority: 'user', animated: true });
                showScrollButton = false;
                isUserScrolling = false;
            }, 150);
        }
    }

    // Group messages by time and role
    interface MessageGroup {
        messages: Message[];
        role: 'user' | 'assistant' | 'system';
        timestamp: Date;
    }

    let messageGroups = $derived(() => {
        if (!messages || messages.length === 0) return [];

        const groups: MessageGroup[] = [];
        let currentGroup: Message[] = [];

        for (const message of messages) {
            if (currentGroup.length === 0) {
                currentGroup.push(message);
            } else {
                const lastMessage = currentGroup[currentGroup.length - 1];
                const timeDiff = message.timestamp.getTime() - lastMessage.timestamp.getTime();
                const sameRole = message.role === lastMessage.role;

                // Group if within 5 minutes and same role
                if (timeDiff < 5 * 60 * 1000 && sameRole) {
                    currentGroup.push(message);
                } else {
                    groups.push({
                        messages: currentGroup,
                        role: currentGroup[0].role,
                        timestamp: currentGroup[0].timestamp
                    });
                    currentGroup = [message];
                }
            }
        }

        if (currentGroup.length > 0) {
            groups.push({
                messages: currentGroup,
                role: currentGroup[0].role,
                timestamp: currentGroup[0].timestamp
            });
        }

        return groups;
    });

    function handleRetry() {
        onRetry?.();
    }

    async function handleCopyMessage(content: string) {
        try {
            await navigator.clipboard.writeText(content);
        } catch (err) {
            console.error('Failed to copy message:', err);
        }
    }

    function formatGroupTime(date: Date): string {
        return new Date(date).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    function getRoleClass(role: string): string {
        return `role-${role}`;
    }

    function getMessageStatusClass(status: string | undefined): string {
        if (status === 'error') return styles.messageError;
        if (status === 'sending') return styles.messageSending;
        return '';
    }

    /**
     * Get thread title by ID.
     */
    function getThreadTitle(threadId: string | undefined): string | undefined {
        if (!threadId) return undefined;
        return threads.find((t) => t.id === threadId)?.title;
    }
</script>

<div class={styles.chatArea}>
    <!-- Messages area -->
    <div bind:this={messagesWrapperRef} class={styles.messagesWrapper} onscroll={handleScroll}>
        {#if messagesLoading === 'loading'}
            <div class={styles.loading}>
                <div class={styles.spinner}></div>
                <span>Loading messages...</span>
            </div>
        {:else if messagesLoading === 'error' || error}
            <div class={styles.error}>
                <svg
                    class={styles.errorIcon}
                    width="48"
                    height="48"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                >
                    <circle cx="12" cy="12" r="10" />
                    <line x1="12" y1="8" x2="12" y2="12" />
                    <line x1="12" y1="16" x2="12.01" y2="16" />
                </svg>
                <p class={styles.errorMessage}>{error || 'Failed to load messages'}</p>
                {#if onRetry}
                    <button class={styles.retryButton} onclick={handleRetry} type="button">
                        Retry
                    </button>
                {/if}
            </div>
        {:else if messages.length === 0}
            <div class={styles.empty}>
                <div class={styles.emptyHeader}>
                    <svg
                        class={styles.emptyIcon}
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                    >
                        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
                    </svg>
                    <span class={styles.emptyText}>Start a conversation</span>
                </div>
                <p class={styles.emptyHint}>Type a message below to begin</p>
            </div>
        {:else}
            <!-- Message groups -->
            {#each messageGroups() as group}
                <div class={styles.messageGroup}>
                    <div class={styles.groupHeader}>
                        <span class={styles.groupRole}>{group.role}</span>
                        <span class={styles.groupTime}>{formatGroupTime(group.timestamp)}</span>
                    </div>
                    <div class={styles.groupMessages}>
                        {#each group.messages as message (message.id)}
                            <div class={[styles.messageWrapper, message.role === 'user' ? styles.userWrapper : styles.assistantWrapper].filter(Boolean).join(' ')}>
                                <div class={[styles.message, styles[getRoleClass(message.role)], getMessageStatusClass(message.status)].filter(Boolean).join(' ')}>
                                    <!-- Thread indicator -->
                                    {#if message.threadId && getThreadTitle(message.threadId)}
                                        <ThreadIndicator
                                            thread={{ id: message.threadId, title: getThreadTitle(message.threadId)! } as any}
                                            onclick={() => onThreadClick?.(message.threadId!)}
                                            compact
                                        />
                                    {/if}
                                    <div class={styles.messageContent}>
                                        <MarkdownRenderer content={message.content} />
                                    </div>
                                    <!-- Message status indicator -->
                                    {#if message.status === 'sending' && message.content.trim() === ''}
                                        <AnsweringIndicator />
                                    {:else if message.status === 'error'}
                                        <div class={styles.statusIndicator} aria-label="Failed to send">
                                            <button
                                                class={styles.retryIconButton}
                                                onclick={() => onRetryMessage?.(message.id)}
                                                title="Retry sending"
                                                type="button"
                                            >
                                                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                                    <path d="M1 4v6h6" />
                                                    <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
                                                </svg>
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                                <!-- Message actions -->
                                {#if message.status !== 'sending'}
                                    <div class={styles.actionsContainer}>
                                        <MessageActions
                                            showCopy={true}
                                            showEdit={message.role === 'user'}
                                            showDelete={true}
                                            showRetry={message.status === 'error'}
                                            showFeedback={message.role === 'assistant'}
                                            onCopy={() => handleCopyMessage(message.content)}
                                            onEdit={() => onEditMessage?.(message.id)}
                                            onDelete={() => onDeleteMessage?.(message.id)}
                                            onRetry={() => onRetryMessage?.(message.id)}
                                        />
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            {/each}

            <!-- Answering indicator -->
            {#if isTyping}
                <AnsweringIndicator />
            {/if}
        {/if}

        <!-- Scroll to bottom button -->
        {#if showScrollButton}
            <button
                class={[styles.scrollToBottom, $isLlmStreaming && styles.pulsing].filter(Boolean).join(' ')}
                onclick={handleScrollToBottom}
                aria-label="Scroll to latest messages"
                type="button"
            >
                <svg
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                >
                    <polyline points="6 9 12 15 18 9" />
                </svg>
            </button>
        {/if}
    </div>

    <!-- Input area -->
    {#if showInput}
        <div class={[styles.inputContainer, inputAnimatingIn && styles.animateIn].filter(Boolean).join(' ')}>
            <ChatInput
                value={inputValue}
                onchange={(v) => (inputValue = v)}
                onsubmit={handleSubmit}
                placeholder="Send a message..."
                loading={loading}
                showIntent={true}
            />
        </div>
    {/if}
</div>
