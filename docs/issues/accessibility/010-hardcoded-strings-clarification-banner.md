# [ISSUE-010] - Hardcoded Strings in Clarification Banner

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/clarification-banner-review-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/ClarificationBanner.svelte`
- **Line Numbers**: TBD

## Description
The ClarificationBanner component contains hardcoded strings that are not marked for translation. This creates accessibility barriers for non-English speaking users and prevents proper internationalization.

## Impact
Accessibility - This fails WCAG guidelines by:
- Creating language barriers for non-English users
- Preventing proper internationalization
- Making the application less accessible globally
- Excluding users who speak languages other than English

## Suggested Fix
Use translation keys instead of hardcoded strings:

```typescript
// Define translation keys
const translations = {
  clarification: {
    en: 'Need clarification?',
    es: '¿Necesitas aclaración?',
    fr: 'Besoin de clarification?',
    de: 'Klärungsbedarf?'
  },
  question: {
    en: 'Ask for more details',
    es: 'Pide más detalles',
    fr: 'Demandez plus de détails',
    de: 'Mehr Details anfragen'
  }
};

// Use reactive language detection
$: clarificationText = translations.clarification[language] || translations.clarification.en;
$: questionText = translations.question[language] || translations.question.en;

// In template
<div class="clarification-banner" aria-label={clarificationText}>
  <span>{clarificationText}</span>
  <button aria-label={questionText}>{questionText}</button>
</div>
```

## Related Issues
- None

## Resolution Status
**DEFERRED** - Not an Accessibility Issue - 2026-03-04

This issue is about internationalization (i18n), not accessibility. The hardcoded strings are in English, which is a language concern, not an accessibility violation.

From an accessibility perspective, the component is properly implemented:
- Buttons have visible text labels
- IconButton components have proper `label` props for screen readers
- Input has placeholder text
- The form is keyboard accessible

Internationalization is a feature enhancement that should be tracked separately when i18n support is added to the application.