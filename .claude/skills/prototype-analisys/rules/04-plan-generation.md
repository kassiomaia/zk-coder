# Plan Generation Rules

These rules govern the structure and content of implementation plans.

## Plan File Location

All implementation plans MUST be generated at:
```
./docs/<prototype-name>/prototype-implementation-plan.md
```

## Plan Status

Every plan MUST have one of three statuses:

| Status | Meaning | Action Required |
|--------|---------|-----------------|
| **READY TO IMPLEMENT** | All questions answered, plan is complete | Proceed with implementation |
| **NOT READY TO IMPLEMENT** | Unresolved questions or issues | Answer questions first |
| **NEEDS CLARIFICATION** | Partial information, requires decisions | User input required |

## Required Plan Sections

### 1. Header
```markdown
# Implementation Plan: <Feature Name>

> **Status:** <STATUS>
> **Generated:** <YYYY-MM-DD>
> **Prototype Source:** prototypes/<prototype-name>/
```

### 2. Summary
Brief description (2-3 sentences) of what will be implemented.

### 3. Clarification Questions (Conditional)
ONLY include if status is NOT "READY TO IMPLEMENT":
```markdown
## Clarification Questions

Before implementation can proceed:

1. [Question 1]
2. [Question 2]
```

### 4. Frontend Implementation

#### Component Architecture
- New Components table
- Components to Extend table
- Reusable Components table

#### State Management
Table mapping prototype state to Svelte patterns

#### CSS Updates
Table listing CSS files and changes

#### Implementation Steps
Numbered checklist of tasks

### 5. Backend Implementation (Conditional)

Include ONLY if backend changes are required:
- API Endpoints table
- Data Models
- Implementation Steps

### 6. File Changes Summary
- New Files list
- Modified Files list
- Files to Review list

### 7. Risks & Considerations
Bulleted list of technical risks

### 8. Acceptance Criteria
Checklist of completion criteria

## Content Rules

### Component Tables
Must include:
- Component name (PascalCase)
- File path (relative to project root)
- Description (one sentence)
- Priority (P1 = critical, P2 = important, P3 = nice-to-have)

### State Management Table
Must include:
- State property name
- TypeScript type
- Svelte pattern ($state, $derived, store)
- Location (component local, stores directory)

### Implementation Steps
- Each step MUST be actionable
- Steps MUST be in dependency order
- Each step MUST be testable
- Use checkbox format: `- [ ] Task description`

## Plan Quality Checklist

Before finalizing the plan, verify:

- [ ] All sections are complete
- [ ] Status accurately reflects readiness
- [ ] All in-scope components are addressed
- [ ] No contextual components are included
- [ ] File paths are correct
- [ ] Dependencies are identified
- [ ] Risks are documented
- [ ] Acceptance criteria are testable

## Plan Updates

When updating an existing plan:
1. Keep original generation date
2. Add "Last Updated" date
3. Document what changed
4. Re-evaluate status
