# [ISSUE-001] - Incomplete Copy Message Actions

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/2026-03-04-MessageActions.svelte.md`
- **Component/File**: `src-ui/src/lib/components/MessageActions.svelte`
- **Line Numbers**: TBD

## Description
The MessageActions component implements copy functionality but doesn't handle all message types correctly. Some message formats (code blocks, markdown tables) are not properly copied, resulting in broken or incomplete copies.

## Impact
Functionality - This affects:
- User experience with message copying
- Data integrity when copying content
- Reliability of the copy feature
- User frustration with broken functionality

## Suggested Fix
Improve copy functionality for all message types:

```typescript
async function copyMessage(message: Message) {
  try {
    let textToCopy: string;

    // Handle different message types
    if (message.type === 'code') {
      textToCopy = message.code;
    } else if (message.type === 'table') {
      textToCopy = formatTableForCopy(message.content);
    } else if (message.type === 'markdown') {
      textToCopy = extractTextFromMarkdown(message.content);
    } else {
      textToCopy = message.content;
    }

    // Use Clipboard API
    await navigator.clipboard.writeText(textToCopy);

    // Provide feedback
    copied = true;
    setTimeout(() => copied = false, 2000);
  } catch (error) {
    console.error('Failed to copy message:', error);
  }
}

function formatTableForCopy(content: string): string {
  // Convert markdown table to plain text table
  return content
    .split('\n')
    .map(line => line.replace(/\\|/g, '|'))
    .join('\n');
}
```

## Related Issues
- None