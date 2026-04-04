# OpenAI Provider

The OpenAI provider is one of the default AI providers supported by the application. It enables integration with OpenAI's GPT models and any OpenAI-compatible API endpoints.

## Configuration

### Required Settings

- **API Key**: Your OpenAI API key (required for authentication)

### Optional Settings

- **Base URL**: Custom API endpoint URL (defaults to `https://api.openai.com/v1`)
  - Used for OpenAI-compatible APIs (e.g., Azure OpenAI, local models)
  - The URL is automatically normalized to remove trailing endpoint paths

## Model Management

### Automatic Model Fetching

The OpenAI provider supports automatic model discovery via the `/models` API endpoint.

**How it works:**
1. When a user configures their API key, the app fetches available models
2. Models are filtered to include only chat models (GPT, O1, O3 series)
3. Non-chat models are excluded (embeddings, audio, legacy models)
4. Fetched models are stored in the local database for offline access

**Model filtering criteria:**
- Included: Models containing `gpt`, `o1`, or `o3` in the name
- Excluded: Models containing `embedding`, `davinci`, `babbage`, `curie`, `ada`, `whisper`, `tts`, `realtime`, or `speech`

### Model Source Modes

| Mode | Description |
|------|-------------|
| **Auto** | Fetch models from API automatically (default) |
| **Manual** | Only use models manually configured by the user |
| **Hybrid** | Combine API-fetched models with manually added ones |

### Manual Model Configuration

If automatic fetching fails or you want to use specific models:

1. Go to Settings > Providers > OpenAI
2. Switch model source to "Manual" or "Hybrid"
3. Click "Add Model"
4. Enter the model name (e.g., `gpt-4o`, `gpt-4-turbo`)
5. Configure capabilities if needed

## Model Capabilities

Capabilities are automatically inferred from model names:

| Capability | Detection Pattern |
|------------|-------------------|
| **Vision** | Models containing `vision` or `gpt-4o` |
| **Audio** | Models containing `whisper`, `tts`, or `audio` |
| **Code** | Enabled by default for all models |
| **Analysis** | Enabled by default for all models |
| **Tips** | Enabled by default for all models |
| **Review** | Enabled by default for all models |

## API Endpoints

The provider uses the following OpenAI API endpoints:

| Endpoint | Purpose |
|----------|---------|
| `POST /chat/completions` | Chat completions (streaming and non-streaming) |
| `GET /models` | List available models |

## Error Handling

When model fetching fails, the system returns a descriptive error suggesting manual configuration:

- **404 Error**: Models endpoint not available
- **Empty Results**: No chat models found in API response
- **Auth Error**: Invalid API key

In all cases, users are guided to manually configure their models.

## Implementation Details

### Key Files

- `src-tauri/src/infrastructure/llm/openai/client.rs` - OpenAI client implementation
- `src-tauri/src/commands/config/api_keys.rs` - API key validation and model fetching commands
- `src-tauri/src/commands/config/models.rs` - Manual model and provider model-source commands
- `src-tauri/src/infrastructure/sqlite/repositories/config/` - SQLite persistence for providers, models, and defaults

### Data Storage

Models are stored in the SQLite database:

- `models` table - All available models (fetched and manual)
- `manual_models` table - User-added models for manual/hybrid mode
- `operation_defaults` table - Model assignments per operation type

## OpenAI-Compatible APIs

The provider supports any OpenAI-compatible API by setting a custom base URL:

- **Azure OpenAI**: Set base URL to your Azure endpoint
- **Local Models**: Set base URL to your local server (e.g., `http://localhost:8000/v1`)
- **Other Providers**: Any provider implementing the OpenAI API specification

Note: Model filtering is based on OpenAI naming conventions. For providers with different naming, use manual model configuration.
