# LiteLLM Streaming Parsing - Research Report

## 1. Chunk Format (All Fields)

LiteLLM emits each streaming update as a JSON chunk following OpenAI's schema:

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion.chunk",
  "created": 1697896000,
  "model": "gpt-4-turbo",
  "system_fingerprint": "fp_xxx",
  "choices": [{
    "index": 0,
    "delta": {
      "role": "assistant",           // First chunk only
      "content": "Hello",            // Text content
      "tool_calls": [...],           // Tool call fragments
      "function_call": {...},        // Deprecated function calls
      "reasoning_content": "...",    // Chain-of-thought (Anthropic)
      "thinking_blocks": [...],      // Claude thinking blocks
      "audio": {...},                // Audio transcription
      "annotations": [...]           // Any annotations
    },
    "finish_reason": null,           // null until final chunk
    "logprobs": null
  }],
  "usage": null,                     // null until usage chunk
  "error": null                      // Present only on error
}
```

## 2. Chunk Types

### Content Chunk
```json
{"choices": [{"delta": {"content": "text"}, "finish_reason": null}]}
```

### Tool Call Chunk
```json
{"choices": [{"delta": {"tool_calls": [{"id": "call_xxx", "type": "function", "function": {"name": "func_name", "arguments": "{\"arg\":"}}]}}]}
```

### Thinking Blocks Chunk (Claude)
```json
{"choices": [{"delta": {"thinking_blocks": [{"type": "thinking", "thinking": "Let me solve..."}], "reasoning_content": "reasoning text"}}]}
```

### Final Content Chunk
```json
{"choices": [{"delta": {}, "finish_reason": "stop"}]}
```

### Usage Chunk (Empty Choices!)
```json
{"id": "...", "choices": [], "usage": {"prompt_tokens": 100, "completion_tokens": 50, "total_tokens": 150}}
```

### Error Chunk (No Choices!)
```json
{"error": {"message": "Model timeout exceeded", "type": "timeout_error", "code": "model_timeout"}}
```

## 3. Error Handling

**Critical**: Error chunks have NO `choices` array, only `error` object:

```typescript
// Correct error detection
if (chunk.error) {
  throw new Error(chunk.error.message);
}
if (!chunk.choices || chunk.choices.length === 0) {
  // Could be usage chunk OR error - check for usage
  if (chunk.usage) {
    // Usage chunk - process token counts
  }
  return; // Skip processing
}
```

## 4. Stream End Detection

1. **finish_reason**: Watch for non-null value (`"stop"`, `"length"`, `"tool_calls"`)
2. **[DONE] sentinel**: Final SSE line `data: [DONE]`
3. **Error chunk**: `chunk.error` present

```typescript
async for (chunk of stream) {
  if (chunk.error) break;
  if (chunk.choices?.[0]?.finish_reason) {
    // Final content chunk
  }
  if (!chunk.choices?.length && chunk.usage) {
    // Usage chunk - stream is ending
  }
}
```

## 5. Provider Normalization

LiteLLM normalizes all providers to OpenAI format:

| Provider | Original | LiteLLM Normalized |
|----------|----------|-------------------|
| Anthropic | `end_turn` | `stop` |
| Vertex AI | `STOP` | `stop` |
| Bedrock | `endTurn` | `stop` |
| Anthropic | `content[0].text` | `choices[0].delta.content` |
| All | tool_use blocks | `tool_calls` array |

## 6. Known Edge Cases

1. **Google Gemini**: May yield only one content chunk then stop (GitHub #4339)
2. **Claude 3.7 thinking**: May drop `thinking_blocks` in intermediate chunks (#10328)
3. **Groq JSON mode**: Does NOT stream at all (#4804)
4. **Ollama Chat Tools**: Streaming tool calls may have issues (#6135)

## 7. Recommended Parsing Logic

```typescript
function processChunk(chunk: LiteLLMChunk) {
  // 1. Check for error
  if (chunk.error) {
    return { type: 'error', error: chunk.error };
  }
  
  // 2. Check for usage (empty choices)
  if (!chunk.choices?.length) {
    if (chunk.usage) {
      return { type: 'usage', usage: chunk.usage };
    }
    return { type: 'empty' }; // Skip
  }
  
  const choice = chunk.choices[0];
  const delta = choice.delta;
  
  // 3. Check for tool calls
  if (delta.tool_calls?.length) {
    return { type: 'tool_call', tool_calls: delta.tool_calls };
  }
  
  // 4. Check for thinking blocks
  if (delta.thinking_blocks?.length || delta.reasoning_content) {
    return { type: 'thinking', thinking_blocks: delta.thinking_blocks, reasoning_content: delta.reasoning_content };
  }
  
  // 5. Content chunk
  if (delta.content !== undefined) {
    return { type: 'content', content: delta.content, finish_reason: choice.finish_reason };
  }
  
  // 6. Final chunk (empty delta with finish_reason)
  if (choice.finish_reason) {
    return { type: 'final', finish_reason: choice.finish_reason };
  }
  
  return { type: 'unknown' };
}
```

## Sources

- [LiteLLM Streaming Docs](https://docs.litellm.ai/stream)
- [LiteLLM Provider Integrations](https://deepwiki.com/BerriAI/litellm/2.3-provider-integrations)
- [SaaS LiteLLM Streaming Architecture](https://gittielabs.github.io/SaasLiteLLM/reference/streaming-architecture/)
- [LiteLLM GitHub Issues](https://github.com/BerriAI/litellm/issues)
