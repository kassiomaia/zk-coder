# Streaming Advanced Concurrency Model - Implementation Progress

## Overview

Implementation of a priority-based scroll concurrency model to fix UI performance issues during streaming.

## Status Summary

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Remove CSS scroll-behavior smooth | ✅ Complete |
| 2 | Create DOM Batching Utility | ✅ Complete |
| 3 | Create Priority Queue | ✅ Complete |
| 4 | Create Live Target Animation | ✅ Complete |
| 5 | Create Content Update Throttler | ✅ Complete |
| 6 | Create Scroll State Machine | ✅ Complete |
| 7 | Create Unified Scroll Controller | ✅ Complete |
| 8 | Integrate into ChatArea | ⏳ Pending |
| 9 | Integrate Throttler into Chat Store | ⏳ Pending |
| 10 | Update Documentation | ⏳ Pending |

---

## Phase 1: Remove CSS scroll-behavior smooth

**Status:** ✅ Complete

**Goal:** Remove `scroll-behavior: smooth` from `.messagesWrapper` to eliminate double-animation conflict.

**File modified:** `src-ui/src/lib/components/chat/ChatArea.module.css`

**Changes made:**
- Removed `scroll-behavior: smooth;` from `.messagesWrapper` class
- Added comment explaining why: "scroll-behavior removed - using custom RAF animations for precise control"

---

## Phase 2: Create DOM Batching Utility

**Status:** ✅ Complete

**Goal:** Create utility to batch DOM reads and writes to prevent layout thrashing.

**File created:** `src-ui/src/lib/utils/domBatcher.ts`

**Features implemented:**
- `scheduleRead(fn, threadId?)` - Queue a DOM read operation
- `scheduleWrite(fn, threadId?)` - Queue a DOM write operation
- `scheduleReadThenWrite(readFn, writeFn, threadId?)` - Convenience for read-then-write patterns
- `measureScrollState(element, threshold?)` - Async batched scroll measurement (returns Promise)
- `measureScrollStateSync(element, threshold?)` - Synchronous scroll measurement (use sparingly)
- `applyScrollPosition(element, scrollTop)` - Batched scroll position write
- `cancelOperation(id)` - Cancel specific operation
- `cancelThreadOperations(threadId)` - Cancel all operations for a thread
- `clearAll()` - Clear all pending operations
- `hasPendingOperations()` - Check if operations are pending
- `getPendingCount()` - Get count of pending reads/writes

**Key design decisions:**
- All reads are processed before writes in each animation frame
- Uses requestAnimationFrame for optimal timing
- Thread ID support for cancellation by thread
- Both async and sync scroll measurement available

---

## Phase 3: Create Priority Queue for Scroll Operations

**Status:** ✅ Complete

**Goal:** Implement priority-based queue with preemption support.

**File created:** `src-ui/src/lib/utils/scrollPriorityQueue.ts`

**Features implemented:**
- Priority levels: `user` (3) > `programmatic` (2) > `background` (1)
- `enqueue(action, options)` - Add operation with priority, returns preemption info
- `cancel(id)` - Cancel specific operation
- `cancelThread(threadId)` - Cancel all operations for thread
- `getNext(threadId)` - Get next operation to execute (removes from queue)
- `complete(id)` - Mark operation as complete
- `hasPending(threadId)` - Check for pending operations
- `isExecuting(threadId)` - Check if operation is executing
- `getExecuting(threadId)` - Get currently executing operation
- `getHighestPriority(threadId)` - Get highest pending priority
- `wouldPreempt(threadId, priority)` - Check if new op would preempt
- `onPriorityChange(listener)` - Listen for user priority additions
- `clearAll()` - Clear all operations
- `getDebugInfo(threadId)` - Debug information

**Key design decisions:**
- Higher priority operations automatically preempt lower priority ones
- Preemption calls `onCancel` callback on preempted operations
- Queue sorted by priority, then by timestamp (FIFO within same priority)
- Thread-isolated queues prevent cross-thread interference

---

## Phase 4: Create Live Target Animation System

**Status:** ✅ Complete

**Goal:** Replace static-target animation with "chasing" animation that tracks live bottom position.

**File created:** `src-ui/src/lib/utils/liveScrollAnimation.ts`

**Features implemented:**
- `startLiveAnimation(element, options)` - Start chasing animation with priority
- `updateLiveTarget(element)` - Update target during content growth
- `stopById(id)` / `stop(threadId)` - Stop animations
- `stopAll()` - Stop all animations
- `isRunning(threadId)` - Check if animation is running
- `getAnimationPriority(threadId)` - Get priority of running animation
- `scrollToBottomInstant(element)` - Instant scroll (no animation)
- `scrollToBottomLive(element, threadId, priority, onComplete?)` - Convenience method
- `getDebugInfo()` - Debug information

**Key design decisions:**
- Background priority animations continuously recalculate target ("chasing")
- User/programmatic priority animations use static target for predictable behavior
- Uses ease-out-quad easing for smooth deceleration
- 5px threshold for considering scroll "at bottom"
- Default 300ms animation duration
- Thread-isolated animations (one per thread)

---

## Phase 5: Create Content Update Throttler

**Status:** ✅ Complete

**Goal:** Reduce Svelte reactive update frequency during streaming.

**File created:** `src-ui/src/lib/utils/streamThrottler.ts`

**Features implemented:**
- `StreamThrottler` class for instance-based usage
- `push(content)` - Throttle/batch update
- `flush()` - Force immediate flush
- `reset()` - Clear state
- `destroy()` - Cleanup and remove throttler
- `getContent()` - Get total accumulated content
- `getState()` - Get current throttler state

**Thread-based convenience API:**
- `getThreadThrottler(threadId, options?)` - Get/create throttler for thread
- `pushToThread(threadId, content, options?)` - Push to thread's throttler
- `flushThread(threadId)` - Flush thread's throttler
- `resetThread(threadId)` - Reset thread's throttler
- `getThreadContent(threadId)` - Get thread's content
- `removeThreadThrottler(threadId)` - Remove thread's throttler

**Key design decisions:**
- 50ms minimum interval between flushes (20fps feel)
- 100ms maximum buffer time before forced flush
- First chunk flushes immediately for responsive feel
- Content accumulates in buffer between flushes
- Callback-based flush notification

---

## Phase 6: Create Scroll State Machine

**Status:** ✅ Complete

**Goal:** Explicit state management with clear transitions.

**File created:** `src-ui/src/lib/utils/scrollStateMachine.ts`

**States:** `idle` → `streaming` → `user_paused` → `resuming` → `idle`

**Events:**
- `STREAM_START` / `STREAM_END`
- `USER_SCROLL_UP` / `USER_SCROLL_TO_BOTTOM`
- `USER_CLICK_SCROLL_DOWN`
- `USER_SEND_MESSAGE`
- `THREAD_CHANGE`

**Features implemented:**
- `sendEvent(threadId, event)` - Send event to state machine
- `getState(threadId)` - Get current state
- `isInState(threadId, state)` - Check if in specific state
- `isStreamingActive(threadId)` - Check if streaming is active
- `shouldAutoScroll(threadId)` - Check if auto-scroll should be enabled
- `isUserPaused(threadId)` - Check if user has paused
- `resetThread(threadId)` - Reset state for thread
- `onStateChange(listener)` - Listen for state changes
- `getValidTransitions(threadId)` - Get valid transitions from current state
- `getHistory(threadId)` - Get state history for debugging
- `getDebugInfo(threadId)` - Debug information

---

## Phase 7: Create Unified Scroll Controller

**Status:** ✅ Complete

**Goal:** Facade coordinating all subsystems.

**File created:** `src-ui/src/lib/utils/scrollController.ts`

**Features implemented:**
- `registerThreadElement(threadId, element, options)` - Register scrollable element
- `unregisterThread(threadId)` - Unregister element
- `handleStreamingStart(threadId)` - Notify streaming started
- `handleStreamingEnd(threadId)` - Notify streaming ended
- `handleContentUpdate(threadId)` - Handle new content during streaming
- `handleUserScroll(threadId)` - Handle user scroll events
- `handleThreadChange(oldThreadId, newThreadId)` - Handle thread switch
- `scrollToBottom(threadId, options)` - Programmatic scroll
- `isAutoScrollEnabled(threadId)` - Check auto-scroll state
- `getThreadScrollState(threadId)` - Get current scroll state
- `isThreadStreaming(threadId)` - Check if streaming
- `hasPendingOperations(threadId)` - Check for pending ops
- `enableAutoScroll(threadId)` / `disableAutoScroll(threadId)` - Toggle auto-scroll
- `resetScrollLock(threadId)` - Reset scroll state
- `getDebugInfo(threadId)` - Debug information
- `clearAll()` - Cleanup all threads

**Key design decisions:**
- Coordinates DOM Batcher, Priority Queue, Live Animation, and State Machine
- Thread-isolated contexts prevent cross-thread interference
- Automatic state transitions based on scroll events
- Supports both animated and instant scrolling

---

## Phase 8: Integrate into ChatArea Component

**Status:** ⏳ Pending

**Goal:** Replace multiple `$effect()` blocks with unified controller.

**File to modify:** `src-ui/src/lib/components/chat/ChatArea.svelte`

**Changes:**
1. Register element with controller on mount
2. Notify controller of streaming state changes
3. Notify controller of content updates
4. Simplify scroll event handlers

---

## Phase 9: Integrate Throttler into Chat Store

**Status:** ⏳ Pending

**Goal:** Integrate `StreamThrottler` into `sendMessageWithLlm`.

**File to modify:** `src-ui/src/lib/stores/chat.ts`

**Changes:**
- Wrap chunk event handling with throttler
- Reduce reactive update frequency

---

## Phase 10: Update Documentation

**Status:** ⏳ Pending

**Files to modify/create:**
- `docs/auto-scrolling.md` - Update with new architecture
- `docs/streaming-concurrency.md` - New comprehensive documentation

---

## Notes

- Backward compatibility: Keep existing `scrollManager.ts` functions for gradual migration
- Fallback: Can revert by removing controller integration if issues arise
- Incremental rollout: Phases can be implemented independently
