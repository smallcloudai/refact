# GUI Streaming Implementation Fixes - Summary (COMPLETED ✅)

## Overview
Fixed critical misalignments between the GUI streaming parser and the LiteLLM backend for proper handling of server-side tools (like `web_search`) and citations from Anthropic/other providers.

## Problem Statement

### 1. Error Format Mismatch
- **Backend sends**: `{"error": {"message": "...", "type": "..."}}`  (LiteLLM format)
- **GUI expected**: `{"detail": "..."}` (FastAPI format)
- **Impact**: Errors during streaming weren't properly displayed

### 2. Hardcoded Tool Filtering
- **Old approach**: Hardcoded list `["web_search"]` to identify server-executed tools
- **Problem**: Not extensible, doesn't work for other server-side tools
- **Impact**: Any new server-side tool would require code changes

### 3. Missing Citations
- **Issue**: Citations from `provider_specific_fields.citation` weren't extracted or displayed
- **Impact**: Users couldn't see sources for web search results

### 4. Ugly Tool Display
- **Issue**: Server-executed tools showed as: `☁️ **web_search**({"query": "..."}) was called on the cloud`
- **Impact**: Poor UX, redundant information (results already in content)

## Solutions Implemented

### 1. Error Handling (types.ts)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/features/Chat/Thread/types.ts`

**Changes**:
- Added `StreamingErrorChunk` type to handle LiteLLM error format
- Added `isStreamingError()` helper function
- Updated `checkForDetailMessage()` to convert both error formats to `DetailMessage`

```typescript
export type StreamingErrorChunk = {
  error: {
    message: string;
    type: string;
    code?: string;
  };
};

export function checkForDetailMessage(str: string): DetailMessage | false {
  const json = parseOrElse(str, {});
  if (isDetailMessage(json)) return json;
  // Handle LiteLLM error format by converting it to DetailMessage
  if (isStreamingError(json)) {
    return { detail: json.error.message };
  }
  return false;
}
```

### 2. Server-Side Tool Detection (types.ts)
**Changes**:
- Added `isServerExecutedTool()` helper function
- Uses `srvtoolu_` prefix detection instead of hardcoded tool names
- Now works for ANY server-executed tool from ANY provider

```typescript
export function isServerExecutedTool(toolCallId: string | undefined): boolean {
  return toolCallId?.startsWith("srvtoolu_") ?? false;
}
```

### 3. Citations Support

#### A. Type Definitions (types.ts in services/refact)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/services/refact/types.ts`

```typescript
export type WebSearchCitation = {
  type: "web_search_result_location";
  cited_text: string;
  url: string;
  title: string;
  encrypted_index?: string;
};

export interface AssistantMessage extends BaseMessage, CostInfo {
  role: "assistant";
  content: string | null;
  // ... other fields
  citations?: WebSearchCitation[] | null; // NEW
  // ... other fields
}

interface BaseDelta {
  role?: ChatRole | null;
  provider_specific_fields?: {    // NEW
    citation?: WebSearchCitation;
    thinking_blocks?: ThinkingBlock[];
  } | null;
}
```

#### B. Citation Extraction (utils.ts)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/features/Chat/Thread/utils.ts`

**Changes**: Extract citations from `delta.provider_specific_fields.citation` and accumulate them in the assistant message.

```typescript
// In formatChatResponse(), when merging content deltas:
const newCitation = cur.delta.provider_specific_fields?.citation;
const citations = newCitation
  ? [...(lastMessage.citations ?? []), newCitation]
  : lastMessage.citations;

return last.concat([{
  role: "assistant",
  content: (lastMessage.content ?? "") + cur.delta.content,
  citations: citations, // Include accumulated citations
  // ... other fields
}]);
```

#### C. Citation Display (AssistantInput.tsx)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/components/ChatContent/AssistantInput.tsx`

**Changes**:
- Added `citations` prop
- Display citations as clickable links under the message content

```tsx
{citations && citations.length > 0 && (
  <Box pb="2" px="2">
    <Text size="2" weight="medium" color="gray">
      Sources:
    </Text>
    <Flex direction="column" gap="1" mt="1">
      {citations.map((citation, idx) => (
        <Link
          key={idx}
          href={citation.url}
          target="_blank"
          rel="noopener noreferrer"
          size="2"
        >
          {citation.title}
        </Link>
      ))}
    </Flex>
  </Box>
)}
```

#### D. Pass Citations (ChatContent.tsx)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/components/ChatContent/ChatContent.tsx`

```tsx
<AssistantInput
  key={key}
  message={head.content}
  reasoningContent={head.reasoning_content}
  toolCalls={head.tool_calls}
  citations={head.citations}  // NEW
  // ... other props
/>
```

### 4. Improved Tool Display (utils.ts)
**Changes**: Removed ugly inline text appending for server-executed tools since:
- Results are already in the message content
- Citations show the sources properly
- No need for redundant "☁️ **web_search**(...)  was called on the cloud" text

```typescript
// Before:
const ignoredText = ignoredTools.map(tool => 
  `\n---\n\n☁️ **${tool.function.name}**\`(${args})\` was called on the cloud`
).join("");
const updatedContent = message.content + "\n" + ignoredText;

// After:
// Server-executed tools shouldn't show inline tool call info
// The results are already in the content and citations
const updatedContent = message.content;
```

### 5. Updated Tests (utils.test.ts)
**File**: `/home/svakhreev/projects/smc/temp/refact/refact-agent/gui/src/features/Chat/Thread/utils.test.ts`

**Changes**: Updated test tool IDs from `call_123` to `srvtoolu_123` to reflect real-world behavior where server-executed tools have the `srvtoolu_` prefix.

## How It Works Now

### Server-Side Tool Flow (e.g., web_search)

1. **LLM streams thinking blocks**: `{"delta": {"reasoning_content": "...", "thinking_blocks": [...]}}`
2. **Tool call with srvtoolu_ prefix**: `{"delta": {"tool_calls": [{"id": "srvtoolu_01J3dX...", "function": {"name": "web_search"}}]}}`
3. **Provider executes tool** (not the client!)
4. **Citations stream in**: `{"delta": {"provider_specific_fields": {"citation": {type: "web_search_result_location", url: "...", title: "..."}}}}`
5. **Content streams with results**: `{"delta": {"content": "Here's the weather forecast..."}}`
6. **GUI accumulates citations** in `formatChatResponse()`
7. **Post-processing filters out srvtoolu_ tools** in `postProcessMessagesAfterStreaming()`
8. **Display shows**:
   - Clean message content
   - Clickable citation links
   - No ugly "was called on the cloud" text

### Client-Side Tool Flow (e.g., tree, cat)

1. **Tool call with regular prefix**: `{"delta": {"tool_calls": [{"id": "toolu_123", "function": {"name": "tree"}}]}}`
2. **GUI recognizes** it's NOT server-executed (no `srvtoolu_` prefix)
3. **Backend executes tool** via LSP
4. **Tool result returned** as separate message
5. **Display shows** tool call and result normally

## Benefits

1. **Extensible**: Any future server-side tool works automatically if it uses `srvtoolu_` prefix
2. **Better UX**: Clean citations display instead of ugly inline text
3. **Error handling**: Both LiteLLM and FastAPI error formats work
4. **Correct flow**: Server-executed tools never sent to backend for execution
5. **Provider agnostic**: Works with Anthropic, OpenAI, or any provider that follows the pattern

## Testing

Run the test suite to verify changes:
```bash
cd refact-agent/gui
npm test -- utils.test.ts
```

All tests should pass with updated tool IDs using `srvtoolu_` prefix.

## Files Changed

1. `/refact-agent/gui/src/features/Chat/Thread/types.ts` - Error handling, tool detection helper
2. `/refact-agent/gui/src/services/refact/types.ts` - Citations type, AssistantMessage, BaseDelta
3. `/refact-agent/gui/src/features/Chat/Thread/utils.ts` - Citation extraction, tool filtering, display cleanup
4. `/refact-agent/gui/src/components/ChatContent/AssistantInput.tsx` - Citation rendering
5. `/refact-agent/gui/src/components/ChatContent/ChatContent.tsx` - Pass citations prop
6. `/refact-agent/gui/src/features/Chat/Thread/utils.test.ts` - Updated test data with correct tool IDs
