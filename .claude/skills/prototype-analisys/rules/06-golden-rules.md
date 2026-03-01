# Golden Rules

These are the inviolable principles of the prototype analysis skill.

## Rule 1: Never Generate Without Questions

**PRINCIPLE:** Understanding is more important than speed.

**REQUIREMENT:** Always ask clarifying questions before generating an implementation plan.

**RATIONALE:**
- Prevents misunderstandings about scope
- Catches design system conflicts early
- Ensures alignment with project goals
- Reduces rework

**EXCEPTION:** None. Every prototype analysis requires clarification.

---

## Rule 2: Flag All Uncertainties

**PRINCIPLE:** An imperfect plan with flagged issues is better than a confident plan with hidden problems.

**REQUIREMENT:** If any aspect is unclear, mark the plan as "NOT READY TO IMPLEMENT" and list all questions.

**RATIONALE:**
- Prevents implementation of incorrect assumptions
- Creates clear accountability for decisions
- Documents open questions for resolution

**FLAGS TO USE:**
- "NEEDS CLARIFICATION" for unanswered questions
- "DESIGN SYSTEM CONFLICT" for style mismatches
- "SCOPE UNCLEAR" for ambiguous component boundaries
- "BACKEND UNCLEAR" for uncertain backend needs

---

## Rule 3: Prioritize Component Reuse

**PRINCIPLE:** Extend before creating. Share before duplicating.

**REQUIREMENT:**
1. Search for existing components before planning new ones
2. Evaluate each existing component for extension potential
3. Identify opportunities for shared components
4. Document why new components are necessary

**RATIONALE:**
- Reduces code duplication
- Maintains UI consistency
- Decreases maintenance burden
- Follows DRY principles

**REUSE CHECKLIST:**
- [ ] Searched existing components
- [ ] Evaluated extension vs. new creation
- [ ] Identified shared component opportunities
- [ ] Justified new component creation

---

## Rule 4: Enforce Design System Compliance

**PRINCIPLE:** The design system is the source of truth for styling.

**REQUIREMENT:**
1. All planned CSS MUST use design tokens
2. Flag any prototype styles using hardcoded values
3. Propose token alternatives for hardcoded values
4. Never approve hardcoded colors or spacing

**RATIONALE:**
- Ensures visual consistency
- Enables theming support
- Simplifies maintenance
- Supports accessibility

**HARDCODED VALUE HANDLING:**
```markdown
**Issue Found:** Prototype uses `#6366f1` (hardcoded color)
**Recommendation:** Use `var(--color-primary-500)` instead
```

---

## Rule 5: Separate Backend Plans

**PRINCIPLE:** Frontend and backend planning are distinct processes.

**REQUIREMENT:**
1. Backend implementation requires its own analysis
2. Apply the same rigor to backend planning as frontend
3. Link frontend and backend plans
4. Coordinate implementation order

**RATIONALE:**
- Backend has different concerns (data, auth, performance)
- Allows independent review of each plan
- Enables parallel development if needed
- Maintains separation of concerns

**BACKEND DECISION TREE:**
```
Does prototype need data persistence?
├── Yes → Backend plan required
└── No → Does prototype integrate external APIs?
    ├── Yes → Backend plan required
    └── No → Backend likely not required (confirm with user)
```

---

## Rule 6: Respect Scope Boundaries

**PRINCIPLE:** Contextual elements provide visual context, not implementation requirements.

**REQUIREMENT:**
1. Only plan components marked as "in-scope" in prototype README
2. Ignore contextual backdrop elements
3. If scope is unclear, ask for clarification
4. Document scope decisions in plan

**RATIONALE:**
- Prevents scope creep
- Focuses implementation effort
- Respects prototype design intent
- Reduces unnecessary work

**SCOPE VERIFICATION:**
```
For each component identified:
1. Check README for scope classification
2. If marked contextual → Exclude from plan
3. If marked in-scope → Include in plan
4. If unmarked → Ask user for clarification
```

---

## Rule 7: Use Parallel Analysis

**PRINCIPLE:** Efficiency without sacrificing thoroughness.

**REQUIREMENT:**
1. Launch all 4 analysis tasks simultaneously
2. Aggregate results before decision gate
3. Cross-validate findings across tasks

**RATIONALE:**
- Reduces analysis time
- Ensures comprehensive coverage
- Enables cross-validation

---

## Rule 8: ⚠️ MANDATORY Prototype Reference

**PRINCIPLE:** The approved prototype is the source of truth for implementation.

**REQUIREMENT:**
Every implementation plan MUST include:
1. **Prototype Reference Header** - Warning that prototype is source of truth
2. **Prototype Files Reference Table** - Map of prototype files to implementation
3. **Component-to-Prototype Mapping** - Exact line numbers for each component
4. **Pre-Implementation Checklist** - Tasks to complete before coding
5. **Prototype Verification Checklist** - Side-by-side comparison tables

**RATIONALE:**
- Ensures design integrity
- Gives developers exact locations for implementation details
- Provides verification checklist to validate implementation
- Creates clear link between prototype and production code
- Reduces rework from deviations

**VIOLATION:**
If a plan is generated WITHOUT prototype reference sections:
- The plan is INCOMPLETE
- Status must be "NOT READY TO IMPLEMENT"
- Add flag: "MISSING PROTOTYPE REFERENCE"

**THIS RULE MUST NOT BE IGNORED.**

---

## Summary

| Rule | Violation Consequence |
|------|----------------------|
| Never Generate Without Questions | Plan marked invalid |
| Flag All Uncertainties | Hidden issues become bugs |
| Prioritize Component Reuse | Code duplication |
| Enforce Design System Compliance | Visual inconsistency |
| Separate Backend Plans | Coupled architecture |
| Respect Scope Boundaries | Scope creep |
| Use Parallel Analysis | Inefficient workflow |
| **Mandatory Prototype Reference** | **Plan incomplete, design deviation** |

**REMEMBER:** These rules exist to ensure quality implementation plans. Breaking them leads to technical debt and rework.
