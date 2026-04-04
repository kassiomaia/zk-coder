# Issue: Message Feedback Not Integrated into LLM Context

## Summary

Users can like/dislike assistant messages via the UI, but the LLM **never receives this feedback context** in subsequent requests. The feedback is stored but never used.

## Severity

**Medium** - Core feature functionality issue

## Evidence

### 1. Storage and UI Layer (WORKING)

**Backend command** - `src-tauri/src/commands/chat/messages.rs:76-91`:
```rust
pub async fn update_message_feedback(
    id: String,
    feedback: Option<String>,
    state: tauri::State<'_, ChatState>,
) -> Result<(), String> {
    // Parses "like"/"dislike" and calls service.update_message_feedback
}
```

**Database** - Feedback stored in `metadata` JSON column in `chat_messages` table.

**Frontend UI** - `src-ui/src/lib/components/chat/MessageActions.svelte:100-109`:
```svelte
function handleThumbsUp() {
    const newFeedback: MessageFeedback | null = feedback === 'like' ? null : 'like';
    onFeedback?.(newFeedback);
}
```

**Store integration** - `src-ui/src/routes/+page.svelte:701-707` calls `chatStore.updateMessageFeedback()`.

### 2. Helper Functions Exist (UNUSED)

**File**: `src-tauri/src/commands/llm/helpers.rs`

```rust
// Line 9: System prompt explaining feedback system
pub const FEEDBACK_SYSTEM_PROMPT: &str = r"The user may provide feedback (like/dislike) on earlier assistant responses...";

// Line 12: Limit for feedback items
pub const FEEDBACK_LIMIT: usize = 5;

// Lines 59-82: Builds feedback summary string
pub fn build_feedback_summary(
    feedback_messages: &[(ChatMessage, MessageFeedback)],
) -> Option<String> {
    // Formats feedback as "LIKED"/"DISLIKED" with short message IDs
}

// Lines 85-108: Combines system prompt with feedback
pub fn build_system_prompt_with_feedback(
    original_prompt: &str,
    feedback_summary: Option<&str>,
) -> String {
    // Appends feedback summary to system prompt
}
```

### 3. Integration Gap in Orchestrator Loop

**File**: `src-tauri/src/commands/llm/orchestrator/loop_runner.rs:135`

```rust
let system_prompt = request.system_prompt.clone().unwrap_or_default();
// ^^^ PROBLEM: Used directly without feedback integration

let core_request = CoreChatRequest {
    model: model.name.clone(),
    messages: core_messages.clone(),
    system: Some(system_prompt.clone()),  // No feedback context injected
    // ...
};
```

**What's missing**:
1. No call to `load_thread_feedback` to retrieve feedback data
2. No call to `build_feedback_summary` to format feedback
3. No call to `build_system_prompt_with_feedback` to inject into system prompt

### 4. No Tauri Command to Fetch Feedback

There's no `get_message_feedback` or similar command exposed. The `load_thread_feedback_impl` exists in the repository but is not called from any command handler.

## Expected Behavior

When LLM request is made for a thread that has message feedback:

1. Fetch feedback data via `ChatState.service.load_thread_feedback(thread_id, FEEDBACK_LIMIT)`
2. Format via `build_feedback_summary()` 
3. Inject via `build_system_prompt_with_feedback(original, Some(summary))`

## Impact

- User likes/dislikes messages but LLM never learns from this feedback
- Feedback feature becomes non-functional for improving response quality
- Violates documented feature: "Use that feedback to improve your response quality and avoid repeating disliked patterns"

## Files Involved

### Backend
- `src-tauri/src/commands/chat/messages.rs` - OK (update command works)
- `src-tauri/src/commands/llm/helpers.rs` - helpers exist but unused
- `src-tauri/src/commands/llm/orchestrator/loop_runner.rs` - **NEEDS FIX**
- `src-tauri/src/infrastructure/sqlite/repositories/chat/mod.rs` - has `load_thread_feedback_impl`

### Frontend
- `src-ui/src/lib/components/chat/MessageActions.svelte` - OK
- `src-ui/src/lib/stores/chat.ts` - OK

## Recommended Fix

In `loop_runner.rs`, around line 135:

1. Add import for `load_thread_feedback` from `ChatState`
2. After loading history, fetch feedback:
   ```rust
   let feedback_data = if let Some(thread_id) = &request.thread_id {
       chat_state.service.load_thread_feedback(thread_id, FEEDBACK_LIMIT)
           .unwrap_or_default()
   } else {
       Vec::new()
   };
   ```
3. Build feedback summary and inject into system prompt:
   ```rust
   let feedback_summary = build_feedback_summary(&feedback_data);
   let system_prompt = build_system_prompt_with_feedback(
       &request.system_prompt.clone().unwrap_or_default(),
       feedback_summary.as_deref()
   );
   ```

Alternatively, add a new Tauri command `get_thread_feedback` to allow frontend to manage this integration.