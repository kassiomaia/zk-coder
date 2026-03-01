# Agent Coding Input Component

## Overview

> **Note**: This prototype focuses on the **Agent Coding Input Component**. The documentation page layout (sidebar, navigation, documentation sections) shown in the prototype is purely contextual backdrop and not part of the feature scope.

The Agent Coding Input Component is the primary interface for users to submit coding instructions to the Coder AI assistant. It provides a sophisticated text input with special character support, AI provider configuration, mode selection, and file reference capabilities. The component is designed to streamline the interaction between developers and the AI coding assistant, making it easy to provide context, select appropriate AI models, and specify the type of assistance needed.

## User Stories

- As a **developer**, I want to write detailed coding instructions so that the AI can understand and implement my requirements accurately.
- As a **developer**, I want to reference specific files in my codebase so that the AI has the necessary context for its suggestions.
- As a **developer**, I want to select between plan mode and edit mode so that I can control whether the AI creates an implementation plan or makes direct edits.
- As a **developer**, I want to choose different AI providers and models so that I can optimize for cost, speed, or quality based on my task.
- As a **developer**, I want to specify a git branch so that the AI understands the codebase context I'm working in.
- As a **developer**, I want to use quick commands via the command palette so that I can perform common actions efficiently.
- As a **developer**, I want AI assistance to improve my prompt so that I can get better results from the AI.

## Functional Requirements

1. The component shall provide a multi-line text input for entering coding instructions.
2. The component shall support mode selection between "Plan Mode" and "Edit Mode".
3. The component shall provide a dropdown for selecting AI providers (Anthropic, OpenAI, Google, Local).
4. The component shall provide a dropdown for selecting AI models based on the chosen provider.
5. The component shall provide a branch selector for specifying the git branch context.
6. The component shall trigger a file reference palette when the user types `@` in the input.
7. The component shall trigger a command palette when the user types `/` in the input.
8. The component shall display attached files as removable chips.
9. The component shall provide an "Improve with AI" button to enhance the user's prompt.
10. The component shall include a send button that is disabled when the input is empty.
11. The component shall display keyboard shortcuts for common actions (@ files, / commands, Ctrl+Enter to send).

## UI Components

### [Contextual - Not in Scope] Documentation Page Layout
**Purpose:** Provides a realistic backdrop for the component prototype
**Responsibilities:**
- Display component documentation
- Show usage examples
- Present API reference

---

### [IN SCOPE] Agent Input Container
**Purpose:** Wraps all input component elements
**Responsibilities:**
- Container for the entire input component
- Provides border, shadow, and overall styling
- Scope:** IN SCOPE

**Variants:**
- Default: Standard appearance with elevated card design
- Focus: Subtle glow effect when any child element is focused

---

### [IN SCOPE] Mode Selector
**Purpose:** Allows users to toggle between Plan and Edit modes
**Responsibilities:**
- Display two mode buttons side by side
- Highlight the active mode
- Communicate the purpose of each mode (with icons and labels)

**Variants:**
- Default: Both modes visible, one active
- Plan Active: Plan button highlighted with primary accent color
- Edit Active: Edit button highlighted with primary accent color

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Segmented control with two buttons | Initial render |
| Plan Selected | Plan button has active styling | User clicks Plan |
| Edit Selected | Edit button has active styling | User clicks Edit |
| Hover | Subtle background change on hovered button | Mouse enter |

---

### [IN SCOPE] Provider Dropdown
**Purpose:** Select AI provider (Anthropic, OpenAI, Google, Local)
**Responsibilities:**
- Display current provider selection
- Show dropdown menu with available providers
- Update model dropdown when provider changes

**Variants:**
- Closed: Shows current provider with chevron icon
- Open: Dropdown menu with all providers, selected provider highlighted

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Button with provider name | Default |
| Open | Dropdown menu visible | User clicks button |
| Provider Selected | Highlighted row with checkmark | User selects provider |

---

### [IN SCOPE] Model Dropdown
**Purpose:** Select AI model within chosen provider
**Responsibilities:**
- Display current model selection
- Show dropdown menu with available models
- Display model tier badge (Premium, Standard, Fast)

**Variants:**
- Closed: Shows current model with chevron icon
- Open: Dropdown menu with models, tiers, and selection indicator

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Button with model name | Default |
| Open | Dropdown menu with models | User clicks button |
| Model Selected | Highlighted row with checkmark | User selects model |

---

### [IN SCOPE] Branch Selector
**Purpose:** Select git branch for context
**Responsibilities:**
- Display current branch name
- Show dropdown with available branches
- Use monospace font for branch names

**Variants:**
- Closed: Shows branch icon and current branch name
- Open: Dropdown menu with branch options

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Button with branch name | Default |
| Open | Dropdown menu with branches | User clicks button |
| Branch Selected | Highlighted row with checkmark | User selects branch |

---

### [IN SCOPE] Improve with AI Button
**Purpose:** Enhance user's prompt using AI
**Responsibilities:**
- Trigger AI prompt improvement
- Show loading state during processing
- Display improved text in input

**Variants:**
- Default: Button with sparkle icon and label
- Loading: Icon spinning, button disabled
- Disabled: Reduced opacity when input is empty

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Primary accent background with icon | Input has text |
| Loading | Spinning icon, disabled | Processing improvement |
| Disabled | Muted opacity | Input empty |
| Hover | Slightly brighter background | Mouse enter |

---

### [IN SCOPE] Input Textarea
**Purpose:** Multi-line text input for coding instructions
**Responsibilities:**
- Accept multi-line text input
- Show placeholder label when empty
- Handle special characters (@ and /) for palettes
- Support keyboard navigation (arrow keys, Enter, Escape)

**Variants:**
- Empty: Shows "Ask Coder to write code..." label
- With Text: User's instruction text
- With Cursor Near Trigger: Shows appropriate palette

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Empty | Placeholder label visible | No input |
| Focused | Remove placeholder when typing | User types |
| With Content | User's text visible | Text entered |
| @ Trigger | File palette appears | User types @ |
| / Trigger | Command palette appears | User types / |

---

### [IN SCOPE] File Attachment Chips
**Purpose:** Display attached files referenced in instruction
**Responsibilities:**
- Show file name with icon
- Allow removal of individual files
- Display in horizontal scrollable list

**Variants:**
- Multiple: Multiple chips in a row
- Single: Single chip

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Chip with file name and remove button | File attached |
| Hover | Remove button becomes visible | Mouse enter on chip |
| Removing | Fade out animation | User clicks remove |

---

### [IN SCOPE] Command Palette (Files)
**Purpose:** Search and select files to reference
**Responsibilities:**
- Display available files filtered by query
- Show file type/extension
- Highlight selected item
- Support keyboard navigation

**Variants:**
- Empty Query: Show all available files
- With Query: Filter files by name
- No Results: Show "No files found" message

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Not visible | Default |
| Open | Dropdown above input | User types @ |
| Item Selected | Highlighted row | User hovers or uses arrow keys |
| Item Chosen | File added to input, palette closes | User clicks or presses Enter |

---

### [IN SCOPE] Command Palette (Commands)
**Purpose:** Search and select commands
**Responsibilities:**
- Display commands grouped by category
- Show command description and shortcut
- Highlight selected item
- Support keyboard navigation

**Variants:**
- Empty Query: Show all commands grouped
- With Query: Filter commands by name/description
- No Results: Show "No commands found" message

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Not visible | Default |
| Open | Dropdown above input | User types / |
| Item Selected | Highlighted row | User hovers or uses arrow keys |
| Item Chosen | Command inserted, palette closes | User clicks or presses Enter |

---

### [IN SCOPE] Input Footer
**Purpose:** Display hints and send button
**Responsibilities:**
- Show keyboard shortcut hints
- Provide send button
- Disable send when input is empty

**Variants:**
- Default: Hints visible, send button disabled
- Ready to Send: Send button enabled

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Disabled send button | Empty input |
| Ready | Enabled send button | Input has text |
| Hover | Send button with glow effect | Mouse enter on send button |

---

### [IN SCOPE] Send Button
**Purpose:** Submit the instruction to the AI
**Responsibilities:**
- Submit instruction with all context
- Show loading state during send
- Disable when input is empty

**Variants:**
- Disabled: Muted appearance, not clickable
- Default: Primary accent color with send icon
- Loading: Spinner animation (not shown in prototype)

**States:**
| State | Appearance | Trigger |
|-------|------------|---------|
| Disabled | Reduced opacity, no hover effect | Empty input |
| Default | Full color, interactive | Input has text |
| Hover | Glow effect | Mouse enter |
| Active | Slightly darker | Mouse down |
| Loading | Loading indicator | Sending (not in prototype) |

## Interaction Flows

### Flow 1: Basic Instruction Submission
1. User types instruction in the text area
2. Send button becomes enabled
3. User clicks send button
4. Instruction is submitted with current configuration (mode, provider, model, branch)

### Flow 2: File Reference with @
1. User types `@` in the text area
2. File palette appears showing available files
3. User types to filter files or uses arrow keys to navigate
4. User selects a file by clicking or pressing Enter
5. File is added as a chip below the input
6. Command palette closes

### Flow 3: Command Selection with /
1. User types `/` in the text area
2. Command palette appears showing available commands grouped by category
3. User types to filter commands or uses arrow keys to navigate
4. User selects a command by clicking or pressing Enter
5. Command is inserted into the input
6. Command palette closes

### Flow 4: Mode Selection
1. User clicks on a mode button (Plan or Edit)
2. Selected mode becomes active with visual indicator
3. Component state updates to reflect new mode

### Flow 5: Provider and Model Selection
1. User clicks provider dropdown
2. Dropdown menu shows available providers
3. User selects a provider
4. Model dropdown updates to show models for selected provider
5. First model is auto-selected

### Flow 6: Improve with AI
1. User types an instruction
2. User clicks "Improve with AI" button
3. Button shows loading state with spinning icon
4. AI processes and returns improved text
5. Input text is replaced with improved version
6. Brief confirmation shows

## User Journeys and Logic

### Journey 1: Submitting a Basic Instruction

**Description:** User wants to ask the AI to generate a simple function.

**Conditional Logic:**
- IF user types in the textarea THEN the input text updates in real-time
- IF input text is not empty THEN the send button becomes enabled
- IF input text is empty THEN the send button remains disabled
- IF user clicks send button THEN instruction is submitted with current configuration
- IF user presses Ctrl+Enter THEN instruction is submitted (keyboard shortcut)

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Types   │
    │ Instruction  │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Input Not    │──── No ────→ Send Button Disabled
    │ Empty?       │
    └──────┬───────┘
           │ Yes
           ▼
    ┌──────────────┐
    │ Send Button  │
    │ Enabled      │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Clicks  │
    │ Send         │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Submit with  │
    │ Configuration│
    └──────┬───────┘
           │
           ▼
    ┌─────────────┐
    │    [End]    │
    └─────────────┘
```

---

### Journey 2: Referencing Files with @

**Description:** User wants to reference a specific file in their instruction.

**Conditional Logic:**
- IF user types `@` in the textarea THEN the file palette appears above the input
- IF user continues typing after `@` THEN the file list is filtered by the query
- IF user presses arrow keys THEN the selection moves through the file list
- IF user presses Enter on a selected file THEN the file is added as an attachment
- IF user clicks a file THEN the file is added as an attachment
- IF file is attached THEN a chip appears below the input with the file name
- IF user clicks the X on a file chip THEN the file is removed from attachments
- IF user clicks outside the palette THEN the palette closes
- IF user presses Escape THEN the palette closes

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Types @ │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ File Palette │
    │ Appears      │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Types   │──── No Filter ────→ Show All Files
    │ to Filter?   │
    └──────┬───────┘
           │ Yes
           ▼
    ┌──────────────┐
    │ Filter Files │
    │ by Query     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Navigates│
    │ (Arrows/Click)│
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Selects │──── Click/Enter ────→ Add File Attachment
    │ File?        │
    └──────┬───────┘                         │
           │ No                              │
           ▼                                 │
    ┌──────────────┐                         │
    │ User Clicks  │                         │
    │ Outside/Esc  │                         │
    └──────┬───────┘                         │
           │                                 │
           ▼                                 ▼
    ┌──────────────┐                 ┌──────────────┐
    │ Close Palette │                 │ Show File    │
    └──────┬───────┘                 │ Chip Below   │
           │                         └──────┬───────┘
           │                                │
           └────────────────────────────────┘
                    │
                    ▼
             ┌─────────────┐
             │    [End]    │
             └─────────────┘
```

---

### Journey 3: Using the Command Palette with /

**Description:** User wants to quickly select a command to execute.

**Conditional Logic:**
- IF user types `/` in the textarea THEN the command palette appears above the input
- IF user continues typing after `/` THEN the command list is filtered by the query
- IF user presses arrow keys THEN the selection moves through the command list
- IF user presses Enter on a selected command THEN the command is inserted into the input
- IF user clicks a command THEN the command is inserted into the input
- IF user clicks outside the palette THEN the palette closes
- IF user presses Escape THEN the palette closes

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Types / │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Command      │
    │ Palette Shows│
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Types   │──── No Filter ────→ Show All Commands
    │ to Filter?   │
    └──────┬───────┘
           │ Yes
           ▼
    ┌──────────────┐
    │ Filter by    │
    │ Query        │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Navigates│
    │ Categories   │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Selects │──── Click/Enter ────→ Insert Command
    │ Command?     │
    └──────┬───────┘                         │
           │ No                              │
           ▼                                 │
    ┌──────────────┐                         │
    │ User Clicks  │                         │
    │ Outside/Esc  │                         │
    └──────┬───────┘                         │
           │                                 │
           ▼                                 ▼
    ┌──────────────┐                 ┌──────────────┐
    │ Close Palette │                 │ Command in   │
    └──────┬───────┘                 │ Input Text   │
           │                         └──────┬───────┘
           │                                │
           └────────────────────────────────┘
                    │
                    ▼
             ┌─────────────┐
             │    [End]    │
             └─────────────┘
```

---

### Journey 4: Improving Prompt with AI

**Description:** User wants AI assistance to improve their instruction.

**Conditional Logic:**
- IF input is empty THEN the "Improve with AI" button is disabled
- IF input has text THEN the "Improve with AI" button is enabled
- IF user clicks "Improve with AI" THEN the button enters loading state
- IF improvement is in progress THEN the button shows a spinning icon and is disabled
- IF improvement completes THEN the input text is replaced with improved version
- IF improvement fails THEN an error message is shown (not in prototype)
- IF improvement succeeds THEN a brief confirmation is shown (subtle)

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Has     │──── Empty ────→ Button Disabled
    │ Input Text?  │
    └──────┬───────┘
           │ Has Text
           ▼
    ┌──────────────┐
    │ Improve Btn  │
    │ Enabled      │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Clicks  │
    │ Improve      │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Button Shows │
    │ Loading State│
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ AI Processes │
    │ Improvement  │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Success?     │──── No ────→ Show Error (not in prototype)
    └──────┬───────┘
           │ Yes
           ▼
    ┌──────────────┐
    │ Replace Input│
    │ with Improved│
    │ Text         │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Brief        │
    │ Confirmation │
    └──────┬───────┘
           │
           ▼
    ┌─────────────┐
    │    [End]    │
    └─────────────┘
```

---

### Journey 5: Selecting Provider and Model

**Description:** User wants to change the AI provider and model.

**Conditional Logic:**
- IF user clicks provider dropdown THEN dropdown menu opens showing available providers
- IF user selects a provider THEN the dropdown closes and selected provider updates
- IF provider changes THEN the model dropdown updates to show models for new provider
- IF provider changes THEN the first model of new provider is auto-selected
- IF user clicks model dropdown THEN dropdown menu opens showing available models
- IF user selects a model THEN the dropdown closes and selected model updates

**Flow Diagram:**
```
    ┌─────────────┐
    │   [Start]   │
    └──────┬──────┘
           │
           ▼
    ┌──────────────┐
    │ User Clicks  │
    │ Provider     │
    │ Dropdown     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Show         │
    │ Providers    │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ User Selects │
    │ Provider     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Update       │
    │ Selected     │
    │ Provider     │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Load Models  │
    │ for Provider │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐
    │ Auto-select  │
    │ First Model  │
    └──────┬───────┘
           │
           ▼
    ┌──────────────┐──── Want Different Model? ──→ Click Model Dropdown
    │ User Can     │                                      │
    │ Click Send   │                                      ▼
    └──────┬───────┘                              Select New Model
           │                                              │
           └──────────────────────────────────────────────┘
                           │
                           ▼
                    ┌─────────────┐
                    │    [End]    │
                    └─────────────┘
```

## Visual Design Specifications

### Layout
- **Component Container:** Elevated card with rounded corners (var(--radius-xl)), subtle border, and shadow
- **Header:** Horizontal flex layout with mode selector on left, controls in center/right, improve button on right
- **Input Area:** Full-width textarea with floating label behavior
- **File Chips:** Horizontal scrollable list below textarea when files are attached
- **Command Palette:** Absolute positioned above the input, full width of container
- **Footer:** Horizontal flex with hints on left, send button on right

### Typography
- **Font Family:** Inter (sans-serif) for UI elements, JetBrains Mono (monospace) for code/file names
- **Font Sizes:**
  - Labels/Buttons: var(--text-sm) = 0.875rem
  - Input text: var(--text-base) = 1rem
  - Hints/Shortcuts: var(--text-2xs) = 0.625rem
- **Font Weights:**
  - Normal text: var(--font-normal) = 400
  - Buttons/Labels: var(--font-medium) = 500
  - Headers: var(--font-semibold) = 600

### Colors
- **Backgrounds:**
  - Container: var(--color-surface)
  - Header/Footer: var(--color-bg-secondary)
  - Input: Transparent (shows container background)
  - Dropdowns: var(--color-bg-elevated)
- **Text:**
  - Primary: var(--color-text-primary)
  - Secondary: var(--color-text-secondary)
  - Tertiary/Muted: var(--color-text-tertiary), var(--color-text-muted)
  - Accent: var(--color-accent-primary)
- **Borders:**
  - Container: var(--color-border-primary)
  - Dropdowns/Buttons: var(--color-border-subtle)
  - Focus: var(--color-border-focus) = var(--color-primary-500)
- **Interactive States:**
  - Hover: var(--color-surface-hover)
  - Selected: var(--color-interactive-selected)
  - Active: var(--color-interactive-selected-hover)

### Spacing
- **Container Padding:** var(--space-3) var(--space-4) for header/footer
- **Input Padding:** var(--space-4)
- **Gap between controls:** var(--space-3)
- **Button Padding:** var(--space-2) var(--space-3)
- **Chip Padding:** var(--space-1-5) var(--space-2-5)

### Shadows
- **Container:** var(--shadow-lg)
- **Dropdowns/Palette:** var(--shadow-xl)
- **Focus Glow:** var(--shadow-glow-sm)

### Transitions
- **Default:** var(--transition-fast) = 150ms ease
- **Hover States:** var(--transition-fast) ease
- **Palette Animation:** slideUp keyframe with var(--ease-out)

## States and Variants

### Agent Input Container States
| State | Appearance | Trigger |
|-------|------------|---------|
| Default | Elevated card with border | Initial render |
| Focus | Subtle glow effect on focus ring | Any child focused |
| Disabled | Reduced opacity, no interaction | Conditional |

### Mode Selector States
| State | Appearance | Trigger |
|-------|------------|---------|
| Plan Active | Plan button has primary accent background | User selects Plan mode |
| Edit Active | Edit button has primary accent background | User selects Edit mode |
| Button Hover | Subtle background change | Mouse hover |
| Button Focus | Focus ring visible | Keyboard navigation |

### Input Textarea States
| State | Appearance | Trigger |
|-------|------------|---------|
| Empty | Placeholder label visible | No text |
| With Text | User text visible, label hidden | User types |
| Focused | Focus ring (via container) | Tab or click |
| @ Trigger | File palette appears above | User types @ |
| / Trigger | Command palette appears above | User types / |

### Command Palette States
| State | Appearance | Trigger |
|-------|------------|---------|
| Closed | Not visible | Default |
| Opening | Fade in + slide up animation | Trigger character typed |
| Open | Full dropdown visible | Animation complete |
| Item Hovered | Item background highlighted | Mouse hover or arrow key |
| Closing | Fade out + slide down | Selection or Escape |

### Send Button States
| State | Appearance | Trigger |
|-------|------------|---------|
| Disabled | Muted, no hover effect | Empty input |
| Enabled | Full color, interactive | Input has text |
| Hover | Glow effect + brighter | Mouse enter |
| Active | Slightly darker | Mouse down |
| Loading | Spinner (not in prototype) | Sending |

## Accessibility Requirements

### Keyboard Navigation
- **Tab:** Navigate between interactive elements (mode buttons, dropdowns, textarea, buttons)
- **Shift+Tab:** Navigate backwards through interactive elements
- **Arrow Keys:** Navigate within dropdowns and command palettes
- **Enter:** Select dropdown item, select command palette item, send instruction
- **Escape:** Close dropdowns, close command palettes
- **Ctrl+Enter:** Send instruction (keyboard shortcut)
- **Ctrl+K:** Open command palette (global shortcut)

### Screen Reader Announcements
- **Mode changes:** Announce "Plan mode selected" or "Edit mode selected"
- **Dropdown open/close:** Announce "Provider dropdown opened" with X items available
- **Command palette:** Announce "Command palette opened" with X commands available
- **File attachment:** Announce "File [name] attached"
- **Send enabled/disabled:** Announce state change

### Focus Management
- **Focus Ring:** Visible 2px outline on focused elements using var(--color-border-focus)
- **Focus Trapping:** When command palette is open, keep focus within palette
- **Focus Return:** After closing dropdown/palette, return focus to triggering element
- **Auto-focus:** After file attachment, return focus to textarea

### ARIA Attributes
- **Roles:**
  - `role="listbox"` for command palette
  - `role="option"` for palette items
  - `role="combobox"` for dropdowns
- **States:**
  - `aria-expanded="true/false"` for dropdowns
  - `aria-selected="true/false"` for palette items
  - `aria-disabled="true"` for disabled buttons
- **Labels:**
  - `aria-label` for icon-only buttons
  - `aria-describedby` for additional context

### Color Contrast
- All text meets WCAG AA contrast ratios (4.5:1 for normal text, 3:1 for large text)
- Focus indicators have sufficient contrast (2px outline with focus color)

## Implementation Notes

### Technical Considerations
1. **Textarea Handling:** The component uses a floating label pattern. When the user types, the placeholder label fades out. Special character detection (@ and /) requires cursor position tracking.

2. **Command Palette Positioning:** The palette is absolutely positioned above the input. Need to handle edge cases where palette would extend beyond viewport.

3. **File Attachments:** Files are tracked both in the `attachedFiles` array and as text in the input. Need to sync these representations and handle removal properly.

4. **Dropdown State:** Only one dropdown should be open at a time. Click outside closes all dropdowns.

5. **Model Update:** When provider changes, model selection should reset to the first available model for the new provider.

6. **Improve with AI:** This feature requires an API call to an AI service. The prototype simulates this with a timeout and random improvement selection.

7. **Keyboard Navigation:** Arrow key navigation in command palette needs to handle both flat file lists and grouped command lists with categories.

### State Management
The component uses a centralized state object with the following properties:
- `mode`: Current mode ('plan' or 'edit')
- `inputText`: Text content of the input
- `selectedProvider`: Currently selected AI provider
- `selectedModel`: Currently selected model
- `selectedBranch`: Current git branch
- `attachedFiles`: Array of attached file names
- `commandPaletteOpen`: Boolean for palette visibility
- `commandPaletteType`: 'files' or 'commands'
- `commandQuery`: Current filter query for palette
- `selectedCommandIndex`: Index of selected item in palette
- `improveLoading`: Boolean for AI improvement loading state

### Edge Cases to Handle
1. Empty file list when typing @
2. Empty command list when typing /
3. No search results in palettes
4. Very long file names (truncate with ellipsis)
5. Many attached files (horizontal scroll)
6. Rapid typing of special characters
7. Pasting text with @ or / characters
8. Mobile viewport constraints for palettes

### Performance Considerations
1. Debounce input filtering for large file/command lists
2. Virtual scrolling for palettes if items > 100
3. Lazy loading of model lists per provider
4. Throttle resize handlers for palette positioning

## Out of Scope

The following features are explicitly NOT included in this prototype and are deferred to future iterations:

1. **Real AI Integration:** The "Improve with AI" button simulates AI response with hardcoded improvements.
2. **Send Functionality:** The send button logs to console but does not make actual API calls.
3. **Real-time Branch Detection:** The branch list is hardcoded, not fetched from git.
4. **Dynamic File Scanning:** Available files are hardcoded, not dynamically scanned from project.
5. **Command Execution:** Selecting commands only inserts the text, does not execute the command.
6. **Persistent State:** Component state resets on page reload (no localStorage persistence).
7. **Undo/Redo:** No undo/redo functionality for input text.
8. **Syntax Highlighting:** No syntax highlighting for code in the input.
9. **Multi-line Command History:** No command history or suggestions based on previous inputs.
10. **File Preview:** No preview when hovering over file chips.
