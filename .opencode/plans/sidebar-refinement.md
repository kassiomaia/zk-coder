# Plan: Sidebar Spatial Hierarchy & Progressive Reveal

## Goal
Refine the `ConversationSidebar` to reinforce spatial hierarchy using directional slide-in animations, controlled easing curves, progressive content reveal, and explicit microcopy for navigation state.

## Proposed Changes

### 1. Animation Architecture (CSS)
We will introduce a "controlled easing curve" and a directional slide-in animation.

*   **File:** `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.module.css`
*   **Changes:**
    *   Define a custom easing variable: `--ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);`
    *   Define `@keyframes slideInFromLeft`:
        *   `0%`: `opacity: 0; transform: translateX(-10px);`
        *   `100%`: `opacity: 1; transform: translateX(0);`
    *   Create a `.slide-in-item` utility class (or apply directly to item selector) that uses these.

### 2. Component Updates

#### A. ConversationItem.svelte
*   **File:** `src-ui/src/lib/components/chat/sidebar/ConversationItem.svelte`
*   **Changes:**
    *   Add `style` prop to `Props` interface to accept inline styles (specifically for `animation-delay`).
    *   Apply this `style` to the root `<li>` or button element.
    *   Update `ConversationItem.module.css` to include the base animation properties (duration, fill-mode, easing), setting `opacity: 0` initially to prevent flash of unstyled content.

#### B. ConversationSidebar.svelte
*   **File:** `src-ui/src/lib/components/chat/sidebar/ConversationSidebar.svelte`
*   **Logic:**
    *   **Progressive Reveal:** When iterating through `groupedConversations`, we need a running count of items to calculate the staggered delay.
    *   **Microcopy:** Add a status line in the header area.
        *   *Default (no selection):* "Select a chat to begin"
        *   *Active (selection exists):* "Viewing active conversation" (or similar contextual text).

## Detailed Implementation Steps

1.  **Modify `ConversationItem.svelte` & `.module.css`**
    *   Update props to accept `style`.
    *   In CSS, set `.item` to have `animation: slideIn 0.4s var(--ease-out-expo) forwards`.
    *   Ensure initial state is hidden/offset if not using a "from" keyframe that handles it.

2.  **Modify `ConversationSidebar.module.css`**
    *   Add the keyframes and CSS variables.
    *   Style the new microcopy element (subtle text color, small font size).

3.  **Modify `ConversationSidebar.svelte`**
    *   Implement the global index counter. Since we are iterating over groups *then* items, we might need a derived store or a helper logic in the template block (or just a let variable if Svelte 5 snippets allow it, otherwise a pre-calculation).
    *   *Approach:* We can just calculate the cumulative index in the template if using `{...}` logic, but it's cleaner to map the data first or use a simple counter if the template allows. Since Svelte templates are declarative, we might need to pre-calculate the "global index" for each item in the `groupedConversations` derivation.
    *   Add the microcopy HTML structure under the "Chats" title.

## Verification
*   **Visual:** The sidebar items should cascade in from the left with a smooth easing.
*   **Performance:** Ensure the stagger doesn't cause jank on large lists (limit max delay if necessary).
*   **Accessibility:** Ensure `prefers-reduced-motion` media query disables these animations.
