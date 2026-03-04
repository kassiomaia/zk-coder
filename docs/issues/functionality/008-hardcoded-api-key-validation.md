# [ISSUE-008] - Hardcoded API Key Validation Logic

## Priority
High

## Source
- **Review File**: `docs/reviews/code-reviewer/custom-provider-modal.md`
- **Component/File**: `src-ui/src/lib/components/config/CustomProviderModal.svelte`
- **Line Numbers**: TBD

## Description
The API key validation logic is hardcoded and doesn't support custom providers. This makes the system inflexible and unable to handle different authentication schemes for various AI providers.

## Impact
Functionality - This affects:
- Extensibility for new providers
- Custom provider support
- Authentication flexibility
- Developer experience

## Suggested Fix
Implement configurable validation:

```typescript
interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  authEndpoint?: string;
  apiKeyValidation?: (key: string) => boolean;
  headers?: Record<string, string>;
}

// Define provider configurations
const PROVIDERS: Record<string, ProviderConfig> = {
  openai: {
    id: 'openai',
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com',
    apiKeyValidation: (key) => key.startsWith('sk-'),
    headers: {
      'Authorization': (key) => `Bearer ${key}`
    }
  },
  claude: {
    id: 'claude',
    name: 'Claude',
    baseUrl: 'https://api.anthropic.com',
    apiKeyValidation: (key) => key.startsWith('sk-ant-'),
    headers: {
      'x-api-key': (key) => key,
      'anthropic-version': '2023-06-01'
    }
  }
  // Add custom provider support
  custom: {
    id: 'custom',
    name: 'Custom Provider',
    baseUrl: '', // Will be set by user
    apiKeyValidation: () => true, // No specific validation
    headers: {} // Will be configured by user
  }
};

function validateApiKey(providerId: string, apiKey: string): boolean {
  const provider = PROVIDERS[providerId];
  return provider?.apiKeyValidation?.(apiKey) || false;
}
```

## Related Issues
- None