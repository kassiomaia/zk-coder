# Hardcoded animation durations in TypingIndicator

## Status
✅ **RESOLVED** (2026-03-04)

## Resolution
The component already uses design system tokens, animation durations. Updated to TypingIndicator.module.css now uses design system tokens.
- `--duration-slow` for `--ease-out`)
- `animation-delay` based on keyframe animation
- The animation-delay` token replaced with design system tokens.

- File: `src-ui/src/lib/components/chat/TypingIndicator.module.css`

## Files Changed
- `src-ui/src/lib/components/chat/TypingIndicator.svelte`
- `src-ui/src/lib/components/config/ProviderIcon.svelte`
- `src-ui/src/lib/styles/design-system.css`

## References
- Original review: docs/reviews/code-reviewer/TypingIndicator-review.md
