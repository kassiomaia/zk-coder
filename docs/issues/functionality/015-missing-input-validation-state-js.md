# Missing input validation in state.js

## Metadata
- **Source Review**: docs/reviews/code-reviewer/2026-03-04-state.js-review.md
- **Location**: docs/prototypes/agent-coding-input/js/state.js:388-412
- **Priority**: Medium
- **Category**: Functionality
- **Status**: Open

## Description
File references and commands are inserted without validation, potentially allowing malicious input.

## Impact
Unvalidated input could lead to security vulnerabilities, such as path traversal attacks or injection attacks, and could also cause runtime errors or unexpected behavior.

## Suggested Fix
Validate file paths and sanitize command inputs before insertion.

```javascript
// Example fix for state.js
const validateFilePath = (path) => {
  // Check for path traversal attempts
  if (path.includes('..') || path.includes('./../')) {
    throw new Error('Invalid file path: path traversal not allowed');
  }

  // Check for potentially dangerous characters
  const dangerousChars = ['<', '>', '|', '&', ';', '`', '$', '(', ')'];
  if (dangerousChars.some(char => path.includes(char))) {
    throw new Error('Invalid file path: contains potentially dangerous characters');
  }

  // Check absolute path against allowed directories
  if (path.startsWith('/')) {
    const allowedDirs = ['/src', '/public', '/docs'];
    const isValid = allowedDirs.some(dir => path.startsWith(dir));
    if (!isValid) {
      throw new Error(`Access denied to: ${path}`);
    }
  }

  return true;
};

const validateCommand = (command) => {
  // Check for command injection attempts
  const dangerousPatterns = [
    /;.*\|/g, // Pipeline injection
    /\$\([^)]*\)/g, // Command substitution
    /`.*`/g, // Backtick command execution
    /&&.*\|/g, // Logical operators with pipes
  ];

  for (const pattern of dangerousPatterns) {
    if (pattern.test(command)) {
      throw new Error('Invalid command: potentially dangerous pattern detected');
    }
  }

  // Sanitize input
  return command
    .replace(/[;&|`$]/g, '') // Remove dangerous characters
    .trim();
};

// Usage in insertFileReference or insertCommand methods
const insertFileReference = async (filePath) => {
  try {
    validateFilePath(filePath);
    const sanitizedPath = filePath.trim();
    // Proceed with insertion...
  } catch (error) {
    console.error('File validation failed:', error);
    throw error; // Re-throw to be caught by caller
  }
};
```

## Related Components
- docs/prototypes/agent-coding-input/js/state.js
- Security validation system
- API endpoints (if any)

## References
- Original review: docs/reviews/code-reviewer/2026-03-04-state.js-review.md