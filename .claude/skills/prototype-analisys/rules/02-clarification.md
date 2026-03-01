# Clarification Rules

These rules govern the mandatory clarification process before plan generation.

## Mandatory Questions

Before generating any implementation plan, the following questions MUST be asked:

### 1. Scope Confirmation
```
"The README marks [X, Y, Z] as in-scope and [A, B, C] as contextual.
Should I proceed with this scope understanding?"
```

**Purpose:** Ensure alignment on what needs to be implemented vs. what's just visual context.

### 2. Component Relevance
```
"I've identified these main components: [list].
Are all of these required for the initial implementation, or should some be deferred?"
```

**Purpose:** Validate component prioritization and identify potential phased implementation.

### 3. Backend Requirements
```
"Does this feature require backend changes?
If so, should I create a separate backend implementation plan?"
```

**Purpose:** Determine if backend planning is needed and establish separation of concerns.

### 4. Existing Components
```
"I found similar existing components: [list].
Should I prioritize extending these over creating new ones?"
```

**Purpose:** Encourage code reuse and maintain consistency.

### 5. Design System Alignment
```
"The prototype uses [specific patterns/colors].
Are these aligned with the current design system, or should adjustments be made?"
```

**Purpose:** Identify potential design system conflicts before implementation.

## Conditional Questions

Ask these questions when specific conditions are met:

### State Management (if complex state is detected)
```
"The prototype has complex state management for [feature].
Should this use local component state, a dedicated store, or integrate with an existing store?"
```

### Animations (if animations.css is present)
```
"The prototype includes animations for [elements].
Should these be implemented as CSS animations, or should we use Svelte transitions?"
```

### Third-party Integration (if external dependencies are detected)
```
"The prototype appears to need [external library].
Is this dependency approved for the project?"
```

### Accessibility (if accessibility requirements are complex)
```
"The prototype has these accessibility requirements: [list].
Are there additional accessibility considerations I should be aware of?"
```

## Response Handling

### Full Clarity
If all questions are answered clearly:
- Proceed to Phase 3 (Component Analysis)
- Document answers in the plan

### Partial Clarity
If some questions are unclear:
- Document what is clear
- Mark unclear items in plan's "Clarification Questions" section
- Set plan status to "NOT READY TO IMPLEMENT"

### No Response
If user does not respond to questions:
- Do NOT generate the plan
- Explain that clarification is required before proceeding
- Offer to make assumptions if user prefers (with explicit approval)

## Question Format

- Keep questions concise and specific
- Provide context from the prototype
- Offer options when possible
- Avoid leading questions
- Ask one topic per question
