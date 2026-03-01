# Prototype Validation Rules

These rules govern the quality validation of prototypes before implementation planning.

## Mandatory Quality Checks

### Documentation Requirements

1. **README Scope Definition**
   - MUST include clear scope note distinguishing in-scope from contextual elements
   - MUST define component hierarchy with scope markers
   - SHOULD include component responsibilities table with scope column

2. **User Journey Documentation**
   - MUST include IF/THEN conditional logic for all journeys
   - MUST include ASCII flow diagrams for visual clarity
   - MUST cover error cases and fallbacks

3. **Component Documentation**
   - MUST document all states (default, hover, active, disabled, loading, error)
   - MUST specify accessibility requirements
   - SHOULD note responsive considerations

### Code Quality Requirements

4. **CSS Separation**
   - NO inline `style="..."` attributes on any HTML element
   - NO `<style>` blocks in `index.html`
   - ALL styles in `css/` files

5. **JavaScript Separation**
   - NO `<script>` blocks containing logic in `index.html`
   - NO inline event handlers like `onclick="..."`
   - ALL JavaScript logic in `js/` files
   - Alpine.js directives (`@click`, `x-show`) are acceptable

6. **Icon Usage**
   - NO inline SVG elements in HTML
   - MUST use Lucide Icons library

7. **Design Token Usage**
   - MUST use design system tokens from `design-system.css`
   - NO hardcoded color values (hex, rgb)
   - NO hardcoded spacing values (use `--space-*` tokens)

## Validation Response

If prototype fails validation:

1. List all failing criteria
2. Provide specific examples from the prototype
3. Suggest fixes for each issue
4. Mark plan as "NOT READY TO IMPLEMENT" until issues are resolved

## Validation Exceptions

Exceptions require explicit user approval:
- Prototype is for exploratory purposes only
- Hardcoded values are intentional for testing
- Documentation will be completed post-implementation
