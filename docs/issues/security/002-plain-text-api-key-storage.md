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

## Proposed Solution

Create a secure logger utility (`src-ui/src/lib/utils/logger.ts`) that:

1. **Redacts Sensitive Fields**: Automatically redacts common sensitive field names like `apiKey`, `key`, `token`, `secret`, `password`, `authorization`, etc.

2. **Recursive Deep Redaction**: Traverses nested objects and arrays to ensure all sensitive data is redacted regardless of depth.

3. **Pattern-Based Redaction**: Uses regex patterns to detect and redact API key patterns (e.g., `sk-ant-...`, `AIza...`, `Bearer ...`).

4. **Development Mode Control**: Only logs in development mode to prevent any logging in production.

5. **Component-Prefixed Logging**: Uses a prefix system like `[ConfigModal]` for easy filtering and debugging.

### Implementation Steps

1. Create `src-ui/src/lib/utils/logger.ts` with the secure logger implementation
2. Update `ConfigModal.svelte` to import and use the secure logger
3. Replace all `console.log`, `console.warn`, and `console.error` calls with the secure logger methods

## Resolution Status
DONE

## Implementation Summary

### Files Created
- `src-ui/src/lib/utils/logger.ts` - Secure logger utility that redacts sensitive data

### Files Modified
- `src-ui/src/lib/components/config/ConfigModal.svelte` - Updated to use secure logger

### Changes Made
1. Created a new `logger.ts` utility with the following features:
   - Automatic redaction of sensitive field names (apiKey, token, password, etc.)
   - Pattern-based redaction for API key formats (sk-*, AIza*, Bearer tokens, JWT)
   - Recursive deep redaction for nested objects and arrays
   - Development-mode-only logging for info/warn/debug (production safe)
   - Prefixed logger support via `createLogger()` function

2. Updated ConfigModal.svelte:
   - Replaced all `console.log` with `log.info`
   - Replaced all `console.warn` with `log.warn`
   - Replaced all `console.error` with `log.error`
   - Created component-specific logger using `createLogger('ConfigModal')`

### Security Benefits
- API keys and sensitive configuration data are never logged in plain text
- Production builds will not output info/warn/debug logs at all
- Error logs still function but with sensitive data redacted
- Pattern matching catches even accidental exposure of key formats

### Manual Testing
1. Open the application in development mode
2. Configure an AI provider with an API key
3. Open browser developer tools console
4. Verify that API keys appear as `[REDACTED]` in any log messages
5. In production build, verify that info/warn/debug logs are not shown