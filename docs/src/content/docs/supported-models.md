---
title: Supported Models in Refact.ai
description: Supported Models in Refact.ai
---

## Cloud Version

With Refact.ai, access state-of-the-art models in your VS Code or JetBrains plugin and select the optimal LLM for each task.

### AI Agent Models

Refact.ai supports advanced models with agent capabilities that can autonomously use tools, integrate with your development environment, and handle complex multi-step tasks:

- **GPT-5 Family** - Latest OpenAI models with reasoning, web search, and code interpreter
  - `gpt-5` - Most advanced model with full agent capabilities (400K context)
  - `gpt-5-mini` - Efficient reasoning model (400K context)
  - `gpt-5-nano` - Ultra-efficient option (400K context)
  - `gpt-5.1` - Enhanced version with improved reasoning (400K context)
  - `gpt-5.1-codex` - Code-specialized variant (400K context)

- **GPT-4.1 Family** - Advanced multimodal models (1M context)
  - `gpt-4.1` (default) - Full-featured with agent capabilities
  - `gpt-4.1-mini` - Balanced performance and cost
  - `gpt-4.1-nano` - Most cost-effective option

- **Claude 4.5 Family** - Anthropic's latest with extended thinking (200K context)
  - `claude-sonnet-4-5` - Balanced performance
  - `claude-haiku-4-5` - Fast and efficient
  - `claude-opus-4.5` - Most capable (PRO+ plans only)

- **O-Series** - Reasoning-focused models
  - `o4-mini` - Multimodal reasoning (200K context)
  - `o3-mini` - Compact reasoning model (200K context)
  - `o4-mini-deep-research` - Autonomous web research agent with code execution support (400K context)

- **Google Gemini Models** - Large context multimodal models
  - `gemini-2.5-pro` - Production-ready (1M context)
  - `gemini-2.5-pro-preview` - Preview access (200K context)
  - `gemini-3-pro-preview` - Next-generation preview (200K context)

### Chat Models

All agent models above can be used for chat, plus additional efficient options:

- **GPT-4.1 Family**
  - `gpt-4.1` (default) - Full-featured multimodal model (1M context)
  - `gpt-4.1-mini` - Balanced option (1M context)
  - `gpt-4.1-nano` - Most efficient (1M context)

- **GPT-5 Family**
  - `gpt-5`, `gpt-5-mini`, `gpt-5-nano` - All support chat with reasoning (400K context)
  - `gpt-5.1`, `gpt-5.1-codex` - Enhanced versions (400K context)

- **Claude 4.5 Family**
  - `claude-sonnet-4-5` - Balanced performance (200K context)
  - `claude-haiku-4-5` - Fast responses (200K context)
  - `claude-opus-4.5` - Maximum capability (200K context, PRO+ only)

- **O-Series**
  - `o4-mini`, `o3-mini` - Reasoning models
  - `o4-mini-deep-research` - Autonomous web research (multi-step internet research)

- **Google Gemini**
  - `gemini-2.5-pro`, `gemini-2.5-pro-preview`, `gemini-3-pro-preview` - Large context models

- **DeepSeek Models** (Refact team only)
  - `deepseek-chat` - High-performance chat with tools (64K context)
  - `deepseek-reasoner` - Reasoning-focused model (64K context)

- **Qwen Models** (Refact team only)
  - `Qwen3-235B-A22B` - Large-scale reasoning model (41K context)

### Advanced Reasoning

For select models, click the `💡Think` button to enable advanced reasoning, helping AI better solve complex tasks. Available only in [Refact.ai Pro plan](https://refact.ai/pricing/).

**Models with Extended Thinking/Reasoning:**
- All GPT-5 family models (OpenAI reasoning)
- All O-series models (OpenAI reasoning)
- All Claude 4.5 family models (Anthropic extended thinking)
- DeepSeek Reasoner (DeepSeek reasoning)
- Qwen3-235B-A22B (Qwen reasoning)

### Model Capabilities Overview

| Capability | Description | Supported Models |
|------------|-------------|------------------|
| **Tools/Function Calling** | Models can use external tools and APIs | Most models |
| **Multimodal** | Support for image inputs | GPT-4.1, GPT-5, O4-mini, Claude 4.5, Gemini |
| **Agent Mode** | Autonomous multi-step task handling | GPT-5, GPT-4.1, Claude 4.5, Gemini, DeepSeek |
| **Reasoning** | Advanced problem-solving with chain-of-thought | GPT-5, O-series, Claude 4.5, DeepSeek, Qwen |
| **Web Search** | Integrated web search capabilities | GPT-5 models, o4-mini-deep-research |
| **Code Interpreter** | Execute code in sandboxed environment | o4-mini-deep-research (supporting tool) |
| **Prompt Caching** | Reduced costs for repeated contexts | OpenAI and Anthropic models |

### Pricing Information

All models are available with transparent token-based pricing:
- **Prompt tokens**: Text you send to the model
- **Generated tokens**: Text the model produces
- **Cached tokens**: Previously processed context (discounted)

Models with prompt caching (OpenAI, Anthropic) offer significant cost savings for repeated contexts. Cache read tokens are typically 90% cheaper than regular prompt tokens.

### Code completion models 
- Qwen2.5-Coder-1.5B


## BYOK (Bring your own key)

Refact.ai lets you connect your own API key and use any external LLM, including GPT, Claude, Gemini, Grok, DeepSeek, and others. It's easy: read the guide in our [BYOK Documentation](https://docs.refact.ai/byok/).



## Self-Hosted Version

In Refact.ai Self-hosted, you can choose among 20+ model options — ready for any task. The full lineup (always up-to-date) is in the [Known Models file on GitHub](https://github.com/smallcloudai/refact-lsp/blob/main/src/known_models.rs).


### Completion models 
<table class="full-table">
<thead>
<tr>
<th>Model Name</th>
<th>Fine-tuning support</th>
</tr>
</thead>
<tbody>
<tr>
<td>Refact/1.6B</td>
<td>✓</td>
</tr>
<tr>
<td>Refact/1.6B/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/1b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/1b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/3b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/7b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder/7b/vllm</td>
<td></td>
</tr>
<tr>
<td>starcoder/15b/base</td>
<td></td>
</tr>
<tr>
<td>starcoder/15b/plus</td>
<td></td>
</tr>
<tr>
<td>starcoder2/3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/3b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/7b/base</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/7b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>starcoder2/15b/base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/1.3b/base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/1.3b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/5.7b/mqa-base</td>
<td>✓</td>
</tr>
<tr>
<td>deepseek-coder/5.7b/vllm</td>
<td>✓</td>
</tr>
<tr>
<td>codellama/7b</td>
<td>✓</td>
</tr>
<tr>
<td>stable/3b/code</td>
<td></td>
</tr>
<tr>
<td>wizardcoder/15b</td>
<td></td>
</tr>
</tbody>
</table>

### Chat models
<table class="full-table">
<thead>
<tr>
<th>Model Name</th>
</tr>
</thead>
<tbody>
<tr>
<td>starchat/15b/beta</td>
</tr>
<tr>
<td>deepseek-coder/33b/instruct</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct-finetune</td>
</tr>
<tr>
<td>deepseek-coder/6.7b/instruct-finetune/vllm</td>
</tr>
<tr>
<td>wizardlm/7b</td>
</tr>
<tr>
<td>wizardlm/13b</td>
</tr>
<tr>
<td>wizardlm/30b</td>
</tr>
<tr>
<td>llama2/7b</td>
</tr>
<tr>
<td>llama2/13b</td>
</tr>
<tr>
<td>magicoder/6.7b</td>
</tr>
<tr>
<td>mistral/7b/instruct-v0.1</td>
</tr>
<tr>
<td>mixtral/8x7b/instruct-v0.1</td>
</tr>
<tr>
<td>llama3/8b/instruct</td>
</tr>
<tr>
<td>llama3/8b/instruct/vllm</td>
</tr>
</tbody>
</table>


### Integrations

On a self-hosted mode, you can also configure **OpenAI** and **Anthropic API** integrations.

1. Go to **Model Hosting** page → **3rd Party APIs** section and toggle the switch buttons for **OpenAI** and/or **Anthropic**.

![3rd Party APIs Secction](../../assets/3-party-apis.png)

2. Click the **API Keys tab** to be redirected to the integrations page (or go via **Settings** → **Credentials**)

![Model Hosting Page with Dropdown Expanded](../../assets/api-keys-link.png)

3. Enter your **OpenAI** and/or **Anthropic** key.

:::note
Make sure the switch button is enabled for each API you want to use — API keys won't be used unless activated.
:::
