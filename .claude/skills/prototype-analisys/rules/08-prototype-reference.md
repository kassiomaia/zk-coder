# Prototype Reference Rule

This rule ensures that implementation plans are tied to the approved prototype, preventing deviation from the designed feature.

## Principle

**The approved prototype is the source of truth for implementation.**

Every implementation plan MUST include explicit references to the prototype files, ensuring developers implement the feature exactly as designed.

## Mandatory Plan Sections

Every implementation plan MUST include the following sections at the beginning:

### 1. Prototype Reference Header

```markdown
## ⚠️ MANDATORY: Prototype Reference

**Before implementing ANY component, the developer MUST reference the approved prototype.**

The prototype at `prototypes/<name>/` is the **source of truth** for:
- Visual design and styling
- Component structure and hierarchy
- Interaction patterns and behaviors
- Animation and transition effects
- State management logic
```

### 2. Prototype Files Reference Table

Map each prototype file to its implementation purpose:

```markdown
### Prototype Files Reference

| Prototype File | Purpose | Implementation Reference |
|----------------|---------|-------------------------|
| `index.html` | Component structure | Convert HTML to Svelte |
| `css/components.css` | Component styles | Convert to CSS modules |
| `css/animations.css` | Animations | Add to global/component CSS |
| `js/state.js` | State management | Convert to Svelte 5 runes |
| `README.md` | Feature specification | Reference for requirements |
```

### 3. Component-to-Prototype Mapping

Include exact line numbers for each component:

```markdown
### Component-to-Prototype Mapping

| Component | HTML Location | CSS Location |
|-----------|---------------|--------------|
| ComponentA | index.html:100-150 | components.css:200-280 |
| ComponentB | index.html:155-200 | components.css:285-350 |
```

### 4. Pre-Implementation Checklist

```markdown
### Pre-Implementation Checklist

Before starting implementation, verify:

- [ ] Open `prototypes/<name>/index.html` in browser to see working prototype
- [ ] Read `prototypes/<name>/README.md` for full feature specification
- [ ] Review `prototypes/<name>/css/components.css` for exact styling
- [ ] Review `prototypes/<name>/js/state.js` for state patterns
- [ ] Keep prototype files open for reference during implementation
```

### 5. Prototype Verification Checklist

Include verification sections in Acceptance Criteria:

```markdown
## Prototype Verification Checklist

### Visual Verification
Open the prototype and implemented component side-by-side:

| Component | Prototype States | Implementation Match |
|-----------|------------------|---------------------|
| ComponentA | Default, Hover, Active | [ ] Verified |
| ComponentB | Default, Selected | [ ] Verified |

### Interaction Verification
- [ ] Hover effects match prototype
- [ ] Selection states match prototype
- [ ] Animations match prototype timing

### State Verification
- [ ] State behavior matches prototype
- [ ] Validation states match prototype
```

## Verification Requirements

### Acceptance Criteria Must Include

Every plan must have prototype compliance criteria:

```markdown
### Prototype Compliance (MANDATORY)
- [ ] Visual design matches prototype exactly
- [ ] Component structure follows prototype HTML
- [ ] Hover/active/selected states match prototype
- [ ] Animations match prototype timing and easing
- [ ] Validation states match prototype (info, success, error)
```

## Line Number Format

When referencing prototype code locations:

- **HTML**: `index.html:50-100` (lines 50 to 100)
- **CSS**: `components.css:200-280` (lines 200 to 280)
- **JS**: `state.js:45-78` (lines 45 to 78)

## Why This Matters

1. **Design Integrity**: Ensures the implemented feature matches what was designed and approved
2. **Developer Guidance**: Gives developers exact locations to find implementation details
3. **Quality Assurance**: Provides verification checklist to validate implementation
4. **Traceability**: Creates clear link between prototype and production code
5. **Reduces Rework**: Prevents deviations that require redesign

## Violation

If a plan is generated WITHOUT prototype reference sections:
- The plan is INCOMPLETE
- Status must be "NOT READY TO IMPLEMENT"
- Add flag: "MISSING PROTOTYPE REFERENCE"

## Summary

| Requirement | Mandatory |
|-------------|-----------|
| Prototype Reference Header | ✅ Yes |
| Prototype Files Table | ✅ Yes |
| Component-to-Prototype Mapping with line numbers | ✅ Yes |
| Pre-Implementation Checklist | ✅ Yes |
| Prototype Verification Checklist | ✅ Yes |
| Prototype Compliance in Acceptance Criteria | ✅ Yes |