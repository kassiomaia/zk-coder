<script lang="ts">
    /**
     * ChatArea Component
     * Main chat interface with message display and input.
     */
    import ChatMessagesList from './ChatMessagesList.svelte';
    import ChatInputContainer from './ChatInputContainer.svelte';
    import ChatScrollButton from './ChatScrollButton.svelte';
    import ClarificationBanner from './ClarificationBanner.svelte';
    import type {Message} from '$lib/types/message';
    import type {Thread} from '$lib/types/thread';
    import styles from './ChatArea.module.css';
    import {isLlmStreaming, nlpState, streamingContent} from '$lib/stores/chat';
    import {initNotifications, notifyAssistantComplete} from '$lib/utils/notifications';
    import {configStore} from '$lib/stores/configStore';
    import {cleanupThreadScroll, initializeThreadScroll, saveScrollPosition} from '$lib/utils/scrollManager';
    import {
        handleContentUpdate,
        handleStreamingEnd,
        handleStreamingStart,
        handleUserClickScrollDown as controllerHandleUserClickScrollDown,
        handleUserScroll as controllerHandleUserScroll,
        handleUserSendMessage as controllerHandleUserSendMessage,
        isAutoScrollEnabled as controllerIsAutoScrollEnabled,
        registerThreadElement,
        scrollToBottom as controllerScrollToBottom,
        unregisterThread
    } from '$lib/utils/scrollController';
    import {onMount, tick} from 'svelte';
    import type {ResolutionType} from '$lib/types/nlp';
    import {createLogger} from '$lib/utils/logger';
    import {shouldForceInitialBottomScroll} from './chatAreaScroll';
    import {chatStore} from '$lib/stores';
    import type {AgentStatus} from '$lib/utils/agentStatus';

    const log = createLogger('ChatArea');

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

    /** Empty state context: 'none' (no conv), 'conversation' (conv but no thread), 'thread' (thread selected) */
    emptyContext?: 'none' | 'conversation' | 'thread';

    /** Optional conversation title for empty state context */
    conversationTitle?: string;

    /** Loading state */
    loading?: boolean;

    /** Is assistant typing */
    isTyping?: boolean;

    /** Status for the thread-linked agent session */
    agentStatus?: AgentStatus | null;

    /** Session ID linked to the visible thread */
    agentSessionId?: string | null;

    /** Error message to display */
    error?: string;

    /** Loading state for messages */
    messagesLoading?: 'idle' | 'loading' | 'success' | 'error';

    /** Thread is resolved - disables input */
    threadResolved?: boolean;

    /** Initial model ID from conversation settings */
    initialModelId?: string | null;

    /** Send message handler - can accept content string or object with content and modelId */
    onSendMessage?: (content: string, options?: { modelId?: string }) => void;

    /** Model change handler - called when user selects a different model */
    onModelChange?: (modelId: string) => void;

    /** Open settings handler */
    onOpenSettings?: () => void;

    /** Retry handler */
    onRetry?: () => void;

    /** Edit message handler */
    onEditMessage?: (id: string) => void;

    /** Delete message handler */
    onDeleteMessage?: (id: string) => void;

    /** Retry message handler (for failed messages) */
    onRetryMessage?: (id: string) => void;

    /** Message feedback handler - called when user likes/dislikes a message */
    onMessageFeedback?: (messageId: string, feedback: 'like' | 'dislike' | null) => void;

    /** Thread click handler */
    onThreadClick?: (threadId: string) => void;

    /** Scroll handler */
    onScroll?: (scrollTop: number, scrollHeight: number, clientHeight: number) => void;

    /** Approve action handler */
    onApproveAction?: (sessionId: string, actionId: string) => void;

    /** Deny action handler */
    onDenyAction?: (sessionId: string, actionId: string) => void;

    /** Continue execution handler */
    onContinueExecution?: (sessionId: string) => void;

    /** Replan execution handler */
    onReplanExecution?: (sessionId: string) => void;
  }

  let {
    messages,
    threadId,
    threads = [],
    emptyContext = 'thread',
    conversationTitle,
    loading = false,
    isTyping = false,
    agentStatus = null,
    agentSessionId = null,
    error,
    messagesLoading = 'idle',
    threadResolved = false,
    initialModelId = null,
    onSendMessage,
    onModelChange,
    onOpenSettings,
    onRetry,
    onEditMessage,
    onDeleteMessage,
    onRetryMessage,
    onMessageFeedback,
    onThreadClick,
    onScroll,
    onApproveAction,
    onDenyAction,
    onContinueExecution,
    onReplanExecution
  }: Props = $props();

  // Input state
  let inputValue = $state('');
  let hasInputActivity = $state(false);
  let hasInputFocus = $state(false);

  // Model selection state - initialize from conversation settings or config default
  let selectedModelId = $state<string | null>(null);

  // Initialize model from conversation settings or config default
  $effect(() => {
    if (initialModelId) {
      selectedModelId = initialModelId;
    } else {
      const defaultModel = configStore.getDefault('code');
      if (defaultModel) {
        selectedModelId = defaultModel;
      }
    }
  });

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

  // Track pending one-time bottom positioning when opening a thread
  let pendingInitialBottomScrollThreadId = $state<string | null>(null);

  // Threshold for showing scroll button (larger zone for visibility)
  const SCROLL_BUTTON_THRESHOLD = 100;

  // Watch for streaming state changes using the new controller
  $effect(() => {
    const streaming = $isLlmStreaming;
    const currentThreadId = threadId;
    const ref = messagesWrapperRef;

    log.debug('streaming state effect:', {
      streaming,
      wasStreaming,
      currentThreadId,
      hasRef: !!ref
    });

    if (!ref || !currentThreadId) return;

    // Detect transition from NOT streaming TO streaming
    if (!wasStreaming && streaming) {
      // Streaming just started - notify controller
      log.debug('streaming START detected');
      handleStreamingStart(currentThreadId);
    }

    // Detect transition from streaming to NOT streaming (completion)
    if (wasStreaming && !streaming) {
      // Streaming just completed - notify controller
      log.debug('streaming END detected');
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

  // Handle thread switching - register/unregister with controller
  $effect(() => {
    const currentThreadId = threadId;
    const ref = messagesWrapperRef;

    log.debug('thread registration effect:', {
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
      log.debug('registering thread:', currentThreadId);
      registerThreadElement(currentThreadId, ref);
      initializeThreadScroll(currentThreadId);
    }

    previousThreadId = currentThreadId;

    // For thread selection UX, always open at bottom.
    // We defer the final jump until messages are loaded/rendered.
    if (ref && currentThreadId) {
      // Prevent initial visual flash while loading
      ref.scrollTop = ref.scrollHeight;
      pendingInitialBottomScrollThreadId = currentThreadId;
      lastScrollTop = ref.scrollTop;
      showScrollButton = false;
    }
  });

  // After selected thread messages are loaded and rendered, force instant bottom positioning once.
  $effect(() => {
    const ref = messagesWrapperRef;
    const currentThreadId = threadId;
    const loadingState = messagesLoading;
    const pendingThreadId = pendingInitialBottomScrollThreadId;

    if (!ref || !currentThreadId) return;
    if (
      !shouldForceInitialBottomScroll({
        currentThreadId,
        pendingThreadId,
        messagesLoading: loadingState
      })
    )
      return;

    let cancelled = false;

    void (async () => {
      await tick();
      if (cancelled || pendingInitialBottomScrollThreadId !== currentThreadId) return;

      controllerScrollToBottom(currentThreadId, { priority: 'programmatic', animated: false });
      lastScrollTop = ref.scrollTop;
      showScrollButton = false;
      pendingInitialBottomScrollThreadId = null;
    })();

    return () => {
      cancelled = true;
    };
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

    log.debug('content update effect:', {
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

    // Notify parent
    onScroll?.(ref.scrollTop, ref.scrollHeight, ref.clientHeight);

    // Notify controller of user scroll
    controllerHandleUserScroll(currentThreadId);
  }

  // Manual scroll to bottom (when user clicks the button)
  function handleScrollToBottom() {
    const ref = messagesWrapperRef;
    const currentThreadId = threadId;
    if (!ref || !currentThreadId) return;

    controllerHandleUserClickScrollDown(currentThreadId);
    showScrollButton = false;
    isUserScrolling = false;
    lastScrollTop = ref.scrollTop;
  }

  // Track input activity for empty state transitions
  function handleInputChange(value: string) {
    inputValue = value;
    hasInputActivity = value.length > 0 || hasInputFocus;
  }

  function handleInputFocus() {
    hasInputFocus = true;
    hasInputActivity = true;
  }

  function handleInputBlur() {
    hasInputFocus = false;
    hasInputActivity = inputValue.length > 0;
  }

  function handleModelChange(modelId: string) {
    selectedModelId = modelId;
    // Notify parent to persist the change
    onModelChange?.(modelId);
  }

  // Re-enable auto-scroll and scroll to bottom when user sends a message
  function handleSubmit() {
    const ref = messagesWrapperRef;
    const currentThreadId = threadId;
    const content = inputValue.trim();
    const canSubmit = Boolean(content && onSendMessage);

    if (currentThreadId && canSubmit) {
      controllerHandleUserSendMessage(currentThreadId);
    }

    if (canSubmit && onSendMessage) {
      onSendMessage(content, { modelId: selectedModelId ?? undefined });
      inputValue = '';
      // Keep selected model for subsequent messages in this conversation
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

  // Ambiguity resolution handlers
  function handleResolveAmbiguity(ambiguityId: string, resolution: string, type: ResolutionType) {
    chatStore.resolveAmbiguity(ambiguityId, resolution, type);
  }

  function handleDismissAmbiguity(ambiguityId: string) {
    chatStore.dismissAmbiguity(ambiguityId);
  }

  // Track messages presence for CSS transitions
  let hasMessages = $derived(messages.length > 0);
</script>

<div class={[styles.chatArea, hasMessages && styles.hasMessages].filter(Boolean).join(' ')}>
  <!-- Messages area -->
  <div
    bind:this={messagesWrapperRef}
    class={[styles.messagesWrapper, !hasMessages && styles.isEmptyState].filter(Boolean).join(' ')}
    onscroll={handleScroll}
  >
    <ChatMessagesList
      {messages}
      {threads}
      {messagesLoading}
      {emptyContext}
      {conversationTitle}
      {isTyping}
      {agentStatus}
      {error}
      {hasInputActivity}
      onEditMessage={onEditMessage}
      onDeleteMessage={onDeleteMessage}
      onRetryMessage={onRetryMessage}
      onMessageFeedback={onMessageFeedback}
      onThreadClick={onThreadClick}
      onRetry={onRetry}
      onApproveAction={onApproveAction}
      onDenyAction={onDenyAction}
      onContinueExecution={onContinueExecution}
      onReplanExecution={onReplanExecution}
    />

    <!-- Scroll to bottom button -->
    <ChatScrollButton show={showScrollButton} onclick={handleScrollToBottom} />
  </div>

  <!-- Ambiguity resolution banner -->
  {#if $nlpState.pendingAmbiguities.length > 0}
    <div class={styles.clarificationWrapper}>
      <ClarificationBanner
        ambiguity={$nlpState.pendingAmbiguities[0]}
        onResolve={handleResolveAmbiguity}
        onDismiss={handleDismissAmbiguity}
      />
    </div>
  {/if}

  <!-- Input area - only show when thread is selected -->
  {#if threadId}
    <ChatInputContainer
      bind:value={inputValue}
      onchange={handleInputChange}
      onsubmit={handleSubmit}
      onfocus={handleInputFocus}
      onblur={handleInputBlur}
      placeholder="Send a message..."
      {loading}
      isAgentWorking={isTyping}
      {agentSessionId}
      {threadResolved}
      modelId={selectedModelId}
      onModelChange={handleModelChange}
      onOpenSettings={onOpenSettings}
    />
  {/if}
</div>
