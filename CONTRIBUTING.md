## 🤝 Contributing to Refact Agent
Thanks for your interest in contributing to Refact Agent! We’re an open-source agent build with community — and we’re excited to have you here.

Whether you're fixing a bug, adding a new model, improving the docs, or exploring areas like the MCP catalog — your contributions help shape the future of AI Agents.

## 🌱 How You Can Contribute
There’s no single path to contributing. Here are a few great starting points:

- Try Refact out and open issues when you hit bugs or have feature ideas.
- Add a new model or provider — this guide includes an example of how to do that
- Explore and extend the MCP catalog
- Improve docs

Much of the setup info in this doc applies across different areas — so feel free to contribute where your interest leads you.

## ✨ Got Something Else in Mind?
If you're excited about something that’s not listed here — feel free to reach out on Discord Community (#contribution channel). We're always open to new contributions and ways to improve together.

## 📚 Table of Contents

- [🚀 Quick Start](#-quick-start)
- [🛠️ Development Environment Setup](#️-development-environment-setup)
- [🧠 Adding Chat Models](#-adding-chat-models)
- [⚡ Adding Completion Models](#-adding-completion-models)
- [🔌 Adding New Providers](#-adding-new-providers)
- [🧪 Testing Your Contributions](#-testing-your-contributions)
- [📋 Best Practices](#-best-practices)
- [🐛 Troubleshooting](#-troubleshooting)
- [💡 Examples](#-examples)

## 🚀 Quick Start

Before diving deep, here's what you need to know:

1. **Chat Models** are for conversational AI (like GPT-4, Claude)
2. **Completion Models** are for code completion (preferably FIM models) like qwen-2.5-coder-base, starcoder2 and deepseek-coder
3. **Providers** are services that host these models (OpenAI, Anthropic, etc.)

## 🛠️ Development Environment Setup

### Prerequisites

- **Rust** (latest stable version)
- **Node.js** and **npm** (for React frontend)
- **Chrome/Chromium** (dev dependency)
- **Git**

### Setting Up the Rust Backend (Engine)

```bash
# Clone the repository
git clone https://github.com/smallcloudai/refact.git
cd refact

# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Navigate to the engine directory
cd refact-agent/engine/

# Build the project
cargo build

# Run the engine with your API key
cargo run -- --address-url Refact --api-key <YOUR_CLOUD_API_KEY> --http-port 8001 --lsp-port 8002 --logs-stderr --vecdb --ast --workspace-folder .
```

### Setting Up the React Frontend (GUI)

```bash
# In a new terminal, navigate to the GUI directory
cd refact-agent/gui/

# Install dependencies
npm ci

# Start the development server
npm run dev
```

The frontend will connect to the Rust engine running on port 8001.

## 🧠 Adding Chat Models

Chat models are used for conversational AI interactions. Here's how to add them:

### Step 1: Add to Provider Configuration

For existing providers, edit the appropriate YAML file in `refact-agent/engine/src/yaml_configs/default_providers/`:

```yaml
# Example: anthropic.yaml
running_models:
  - claude-3-7-sonnet-latest
  - claude-3-5-sonnet-latest
  - your-new-model  # Add your model here

chat_models:
  your-new-model:
    n_ctx: 200000
    supports_tools: true
    supports_multimodality: true
    supports_agent: true
    tokenizer: hf://your-tokenizer-path
```
for more info about which config needs to be set up, you can see known_models.json


### Step 2: Test the Model

Once set up, test your model in the Refact frontend:

- Can it call tools?
- Does it support images (if enabled)?
- Do the flags behave as expected?

This ensures everything works smoothly end-to-end.


## ⚡ Adding Completion Models

Completion models are used for code completion. FIM (Fill-in-the-Middle) models work best.

### Step 1: Understand FIM Tokens

FIM models use special tokens:
- `fim_prefix`: Text before the cursor
- `fim_suffix`: Text after the cursor
- `fim_middle`: Where the completion goes
- `eot`: End of text token

### Step 2: Add to Known Models
Add to known models (in json) or provider file (in yaml)
```json
{
  "completion_models": {
    "your-completion-model": {
      "n_ctx": 8192,
      "scratchpad_patch": {
        "fim_prefix": "<|fim_prefix|>",
        "fim_suffix": "<|fim_suffix|>",
        "fim_middle": "<|fim_middle|>",
        "eot": "<|endoftext|>",
        "extra_stop_tokens": [
          "<|repo_name|>",
          "<|file_sep|>"
        ],
        "context_format": "your-format",
        "rag_ratio": 0.5
      },
      "scratchpad": "FIM-PSM",
      "tokenizer": "hf://your-tokenizer-path",
      "similar_models": []
    }
  }
}
```

### Step 3: Test Code Completion

Use the Refact IDE plugin in XDebug mode. It should connect to your local LSP server on port 8001.

Try triggering completions in the IDE to make sure everything’s working as expected.


## 🔌 Adding New Providers

To add a completely new OpenAI-compatible provider:

### Step 1: Create Provider Configuration

Create `refact-agent/engine/src/yaml_configs/default_providers/your-provider.yaml`:

```yaml
chat_endpoint: https://api.your-provider.com/v1/chat/completions
completion_endpoint: https://api.your-provider.com/v1/completions
embedding_endpoint: https://api.your-provider.com/v1/embeddings
supports_completion: true

api_key: your-api-key-format

running_models:
  - your-model-1
  - your-model-2

model_default_settings_ui:
  chat:
    n_ctx: 128000
    supports_tools: true
    supports_multimodality: false
    supports_agent: true
    tokenizer: hf://your-default-tokenizer
  completion:
    n_ctx: 8192
    tokenizer: hf://your-completion-tokenizer
```

### Step 2: Add to Provider List

Edit `refact-agent/engine/src/caps/providers.rs` and add your provider to the `PROVIDER_TEMPLATES` array:

```rust
const PROVIDER_TEMPLATES: &[(&str, &str)] = &[
    ("anthropic", include_str!("../yaml_configs/default_providers/anthropic.yaml")),
    ("openai", include_str!("../yaml_configs/default_providers/openai.yaml")),
    // ... existing providers ...
    ("your-provider", include_str!("../yaml_configs/default_providers/your-provider.yaml")),
];
```

### Step 3: Test Provider Integration

Test should be done in UI to see if it can be set up, and if their models work after that.

## 🧪 Testing Your Contributions

### Unit Tests

```bash
cd refact-agent/engine/
cargo test
```

### Manual Testing Checklist

-  Model appears in capabilities endpoint (`/v1/caps`)
-  Chat functionality works
-  Code completion works (for completion models)
-  Tool calling works (if supported)
-  Multimodality works (if supported)
-  Error handling is graceful
-  Performance is acceptable

### Using xDebug for IDE Testing

Enable xDebug in your IDE extension settings to connect to your locally built Rust binary for testing completion models.

## 📋 Best Practices

### Model Configuration

1. **Context Windows**: Set realistic `n_ctx` values based on the model's actual capabilities
2. **Capabilities**: Only enable features the model actually supports
3. **Tokenizers**: Use the correct tokenizer for accurate token counting
4. **Similar Models**: Group models with similar capabilities

### Provider Configuration

1. **API Keys**: Use environment variables for sensitive data
2. **Endpoints**: Ensure URLs are correct and follow OpenAI compatibility
3. **Error Handling**: Test edge cases and error conditions
4. **Rate Limiting**: Consider provider-specific limitations

### Code Quality

1. **Commit messages**: Use clear, descriptive commit messages

## 🐛 Troubleshooting

### Common Issues

**Model not appearing in capabilities:**
- Ensure provider is properly loaded
- Check that the model has the required capabilities, for example, supports_agent for agentic mode

**Tokenizer errors:**
- Verify tokenizer path is correct
- Use `fake` tokenizer for testing if needed

**API connection issues:**
- Verify endpoint URLs are correct
- Check API key format authorization
- Test with curl directly first

**Completion not working:**
- Ensure FIM tokens are correctly configured
- Check `scratchpad` type is appropriate
- Verify context format matches model expectations

### Debug Commands

```bash

# Test specific endpoints
curl http://127.0.0.1:8001/v1/caps
curl http://127.0.0.1:8001/v1/rag-status

# Validate configuration
cargo check
```

## 💡 Examples

### Example 1: Adding Claude 4 (Hypothetical)

Make sure your model is listed in the config with all required fields — like n_ctx, and any other relevant settings.

-  **Update anthropic.yaml:**
```yaml
chat_models:
  claude-4:
    n_ctx: 200000
    supports_tools: true
    supports_multimodality: true
    supports_agent: true
    supports_reasoning: anthropic
    supports_boost_reasoning: true
    tokenizer: hf://Xenova/claude-tokenizer

  claude-3-7-sonnet-latest:
    n_ctx: 200000
    supports_tools: true
    supports_multimodality: true
    supports_agent: true
    supports_reasoning: anthropic
    supports_boost_reasoning: true
    tokenizer: hf://Xenova/claude-tokenizer
```

### Example 2: Adding a New FIM Model

```json
"new-coder-model": {
  "n_ctx": 16384,
  "scratchpad_patch": {
    "fim_prefix": "<PRE>",
    "fim_suffix": "<SUF>",
    "fim_middle": "<MID>",
    "eot": "<EOT>"
  },
  "scratchpad": "FIM-PSM",
  "tokenizer": "hf://company/new-coder-model"
}
```

### Example 3: Adding a Custom Provider

```yaml
# custom-ai.yaml
chat_endpoint: https://api.anthropic.com/v1/chat/completions
supports_completion: false

api_key: sk-ant-...

chat_models:
  claude-3-7-sonnet-latest:
    n_ctx: 200000
    supports_tools: true
    supports_multimodality: true
    supports_clicks: true
    supports_agent: true
    supports_reasoning: anthropic
    tokenizer: hf://Xenova/claude-tokenizer

model_default_settings_ui:
  chat:
    n_ctx: 200000
    supports_tools: true
    supports_multimodality: true
    supports_agent: true
    tokenizer: hf://Xenova/claude-tokenizer
```

---

## 🎯 Next Steps

1. **Join our [Discord](https://www.smallcloud.ai/discord)** for community support
2. **Check [GitHub Issues](https://github.com/smallcloudai/refact/issues)** for contribution opportunities - search for tags related to good first issues
3. **Check [Documentation](https://docs.refact.ai/)** for more details

Happy contributing! 🚀



