# [ISSUE-002] - Plain Text API Key Storage in Logs

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer-ConfigModal.svelte-2026-03-04.md`
- **Component/File**: `src-ui/src/lib/components/config/ConfigModal.svelte`
- **Line Numbers**: 98, 100, 109, 116, 131, 134, 137

## Description
The ConfigModal component has multiple console.log statements that log API keys and sensitive configuration data in plain text. These logs could expose sensitive information in console outputs, which are accessible to anyone with access to the browser developer tools.

## Impact
Security - This exposes sensitive API keys and configuration data:
- API keys may be visible in console logs
- Sensitive data could be captured by malicious extensions
- Developer tools can be accessed by anyone using the application
- Logs may persist in browser history or crash reports

## Suggested Fix
Replace console.log statements with a secure logging system that filters sensitive data:

```typescript
// Create a logger utility that filters sensitive data
const logger = {
  info: (message: string, data?: any) => {
    // Filter out sensitive data before logging
    const filteredData = data && typeof data === 'object'
      ? { ...data, apiKey: '[REDACTED]' }
      : data;
    console.log(message, filteredData);
  },
  warn: console.warn,
  error: console.error
};

// Replace direct console.log calls
logger.info('Provider selected', { id: provider.id, name: provider.name });
logger.info('API key stored successfully');
```

## Related Issues
- None