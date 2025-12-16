# Refact Agent GUI - Developer Guide

**Last Updated**: December 2024  
**Version**: 2.0.10-alpha.3  
**Repository**: https://github.com/smallcloudai/refact/tree/main/refact-agent/gui

---

## 📋 Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture](#architecture)
3. [Tech Stack](#tech-stack)
4. [Getting Started](#getting-started)
5. [Message Flow & Streaming](#message-flow--streaming)
6. [State Management](#state-management)
7. [UI & Styling](#ui--styling)
8. [API Services](#api-services)
9. [IDE Integration](#ide-integration)
10. [Tool Calling System](#tool-calling-system)
11. [Development Workflows](#development-workflows)
12. [Testing](#testing)
13. [Debugging](#debugging)
14. [Special Features](#special-features)
15. [Common Patterns](#common-patterns)

---

## Project Overview

### What is This?

Refact Chat GUI is a **React-based AI coding assistant** that provides:

- **Real-time streaming chat** with AI models
- **Tool calling** for file operations, shell commands, and IDE integration
- **Multi-host support**: Web, VSCode, JetBrains IDEs
- **Checkpoints system** for workspace rollback
- **Docker container management**
- **Integration configuration** UI

### Key Characteristics

- **Library-first**: Builds to `dist/chat/` (browser UMD) and `dist/events/` (Node.js types)
- **Dual consumption**: Used by IDE extensions AND standalone web UI
- **LSP-centric**: All AI operations go through local LSP server (http://127.0.0.1:8001)
- **Production-ready**: Redux persist, error boundaries, telemetry, compression hints

### Build Outputs

```
dist/chat/index.umd.cjs    # Browser bundle (consumed by IDEs)
dist/chat/index.js         # ES module
dist/chat/style.css        # Bundled styles
dist/events/index.js       # TypeScript types for IDE integrations
```

**Usage in browser:**

```html
<script src="refact-chat-js/dist/chat/index.umd.cjs"></script>
<script>
  RefactChat.render(document.getElementById("root"), {
    host: "web",
    lspPort: 8001,
    features: { statistics: true, vecdb: true, ast: true, images: true },
  });
</script>
```

---

## Architecture

### High-Level Structure

```
┌─────────────────────────────────────────────────────────┐
│                    React Application                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   Features   │  │  Components  │  │    Hooks     │  │
│  │  (Redux)     │  │  (UI Layer)  │  │  (Logic)     │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                  │                  │           │
│         └──────────────────┴──────────────────┘           │
│                            │                               │
│                    ┌───────▼────────┐                     │
│                    │  Services      │                     │
│                    │  RTK Query APIs│                     │
│                    └───────┬────────┘                     │
└────────────────────────────┼──────────────────────────────┘
                             │
            ┌────────────────┼────────────────┐
            │                │                │
     ┌──────▼──────┐  ┌─────▼─────┐  ┌──────▼──────┐
     │ Local LSP   │  │ SmallCloud│  │ IDE (via    │
     │ Server      │  │ Auth API  │  │ postMessage)│
     │ :8001       │  │           │  │             │
     └─────────────┘  └───────────┘  └─────────────┘
```

### Directory Structure

```
gui/
├── src/
│   ├── features/          # Redux slices + feature components
│   │   ├── Chat/          # Core chat logic (Thread/, actions, selectors)
│   │   ├── History/       # Chat history management
│   │   ├── Checkpoints/   # Workspace rollback system
│   │   ├── Config/        # Global configuration
│   │   ├── Integrations/  # Integration management UI
│   │   ├── Providers/     # LLM provider configuration
│   │   └── ...
│   ├── components/        # Reusable UI components
│   │   ├── Chat/          # Chat container
│   │   ├── ChatContent/   # Message rendering
│   │   ├── ChatForm/      # Input form + controls
│   │   ├── Sidebar/       # Navigation
│   │   └── ...
│   ├── hooks/             # Custom React hooks (60+)
│   ├── services/          # API definitions
│   │   ├── refact/        # LSP server APIs (RTK Query)
│   │   └── smallcloud/    # Cloud auth APIs
│   ├── app/               # Redux store setup
│   ├── events/            # IDE integration types
│   ├── lib/               # Library entry point
│   │   └── render/        # Render function + CSS
│   └── utils/             # Utility functions
├── generated/             # GraphQL codegen output
├── public/                # Static assets
└── dist/                  # Build output (git-ignored)
```

### Data Flow Patterns

**1. User Action → State Update → UI Re-render**

```
User clicks "Send"
  → dispatch(chatAskQuestionThunk)
  → sendChat() API call
  → streaming chunks arrive
  → dispatch(chatResponse) per chunk
  → reducer updates state.chat.thread.messages
  → React re-renders ChatContent
```

**2. IDE Integration (postMessage)**

```
IDE Extension ⇄ window.postMessage ⇄ GUI (iframe)
    │                                      │
    ├─ Context updates (active file) ────→│
    │                                      │
    │←──── Commands (open file, paste) ───┤
```

**3. Tool Calling Flow**

```
AI suggests tool_call
  → Confirmation popup (if not automatic)
  → User approves
  → Tool executed (LSP or IDE)
  → Result message inserted
  → AI continues with result
```

---

## Tech Stack

### Core Technologies

| Layer                | Technology                         | Purpose                       |
| -------------------- | ---------------------------------- | ----------------------------- |
| **UI Framework**     | React 18.2                         | Component-based UI            |
| **Language**         | TypeScript 5.8 (strict mode)       | Type safety                   |
| **Build Tool**       | Vite 5.0 + SWC                     | Fast dev server & bundling    |
| **State Management** | Redux Toolkit 2.2                  | Global state + caching        |
| **Data Fetching**    | RTK Query                          | API layer with auto-caching   |
| **GraphQL**          | urql 4.2 (SmallCloud only)         | Auth/user/teams queries       |
| **Styling**          | CSS Modules + Radix Themes         | Scoped styles + design system |
| **UI Components**    | Radix UI                           | Accessible primitives         |
| **Testing**          | Vitest 3.1 + React Testing Library | Unit & integration tests      |
| **Mocking**          | MSW 2.3                            | API mocking for tests/stories |
| **Storybook**        | Storybook 7.6                      | Component development         |

### Key Dependencies

**State & Data**

- `@reduxjs/toolkit` - Modern Redux with `combineSlices`, RTK Query, middleware
- `redux-persist` - Persist chat history to localStorage
- `urql` - GraphQL client (SmallCloud API only, not for chat)
- `uuid` - Generate chat/message IDs

**UI Components**

- `@radix-ui/react-*` - Accordion, Toolbar, Collapsible, Icons
- `@radix-ui/themes` - Design system (colors, spacing, typography)
- `framer-motion` - Animations
- `lottie-react` - Animated icons

**Utilities**

- `react-markdown` + `remark-gfm` + `rehype-katex` - Markdown rendering
- `react-syntax-highlighter` - Code highlighting
- `diff` - Generate diffs for file changes
- `echarts-for-react` - Usage statistics charts
- `react-dropzone` - File upload
- `textarea-caret` - Cursor position (autocomplete)

### Build Configuration

**Vite Config** (`vite.config.ts`)

```typescript
{
  plugins: [react(), eslint(), dts()],
  build: {
    lib: {
      entry: 'src/lib/index.ts',  // Browser bundle
      name: 'RefactChat',
      fileName: 'index'
    },
    outDir: 'dist/chat'
  },
  server: {
    proxy: {
      '/v1': process.env.REFACT_LSP_URL ?? 'http://127.0.0.1:8001'
    }
  }
}
```

**Dual Build**: Separate config for Node.js types (`vite.node.config.ts` → `dist/events/`)

**TypeScript Config**

```typescript
{
  compilerOptions: {
    target: 'ES2020',
    module: 'ESNext',
    moduleResolution: 'bundler',
    strict: true,              // Full strict mode
    jsx: 'react-jsx',
    plugins: [
      { name: 'typescript-plugin-css-modules' },  // CSS typing
      { name: '@0no-co/graphqlsp' }              // GraphQL intellisense
    ]
  }
}
```

**ESLint**: `@typescript-eslint/strict-type-checked` (aggressive type checking)

---

## Getting Started

### Prerequisites

1. **Node.js 18+** (uses ES2020 features)
2. **Refact LSP Server** running on `http://127.0.0.1:8001`
   - Required for chat, tools, caps endpoints
   - Get it: https://github.com/smallcloudai/refact-lsp

### Initial Setup

```bash
# Install dependencies
npm ci

# Start dev server
npm run dev
# → http://localhost:5173

# With custom LSP URL
REFACT_LSP_URL="http://localhost:8001" npm run dev
```

### Environment Variables

| Variable          | Purpose              | Default                 |
| ----------------- | -------------------- | ----------------------- |
| `REFACT_LSP_URL`  | Dev proxy target     | `http://127.0.0.1:8001` |
| `DEBUG`           | Enable debug logging | (unset)                 |
| `REFACT_LSP_PORT` | Runtime LSP port     | `8001`                  |

**Debug mode:**

```bash
DEBUG=refact,app,integrations npm run dev
```

### Available Scripts

```json
{
  "dev": "vite", // Dev server (5173)
  "build": "tsc && vite build && vite build -c vite.node.config.ts",
  "preview": "vite preview", // Preview production build
  "test": "vitest", // Run tests (watch mode)
  "test:no-watch": "vitest run", // CI tests
  "test:ui": "vitest --ui", // Visual test runner
  "coverage": "vitest run --coverage", // Coverage report
  "storybook": "storybook dev -p 6006", // Component explorer
  "build-storybook": "storybook build", // Static storybook
  "lint": "eslint . --ext ts,tsx", // Type-aware linting
  "types": "tsc --noEmit", // Type checking only
  "format": "prettier . --write", // Auto-format
  "generate:graphql": "graphql-codegen", // Generate GraphQL types
  "alpha:publish": "npm publish --tag alpha"
}
```

### First Time Setup Checklist

- [ ] `npm ci` completes successfully
- [ ] LSP server is running (check `http://127.0.0.1:8001/v1/ping`)
- [ ] Dev server starts: `npm run dev`
- [ ] Navigate to `http://localhost:5173`
- [ ] Chat interface loads without errors
- [ ] Can send a test message (requires API key or local model)
- [ ] Storybook works: `npm run storybook`
- [ ] Tests pass: `npm run test:no-watch`

### Project Configuration Files

```
gui/
├── package.json            # Dependencies & scripts
├── tsconfig.json           # TypeScript compiler options
├── tsconfig.node.json      # Node-specific TS config
├── vite.config.ts          # Main Vite config (browser)
├── vite.node.config.ts     # Node types build
├── .eslintrc.cjs           # ESLint rules
├── .prettierrc             # (if exists) Code formatting
├── codegen.ts              # GraphQL code generation
├── .storybook/             # Storybook configuration
│   ├── main.ts
│   └── preview.tsx
└── .husky/                 # Git hooks
    └── pre-commit          # Runs lint-staged
```

**Lint-staged** (pre-commit):

```json
{
  "*.{ts,tsx}": ["prettier --write", "eslint --cache --fix"],
  "*.{js,css,md}": "prettier --write"
}
```

---

## Message Flow & Streaming

### Overview

The chat system uses **Server-Sent Events (SSE)** over HTTP fetch streams for real-time AI responses.

### Complete Flow Timeline

```
1. User types message & clicks Send
   ↓
2. dispatch(chatAskQuestionThunk({messages, chatId, mode}))
   → src/features/Chat/Thread/actions.ts:335
   ↓
3. formatMessagesForLsp(messages)
   → Converts internal format to LSP format
   → Filters out UI-only fields
   ↓
4. sendChat({messages, model, stream: true, abortSignal, ...})
   → src/services/refact/chat.ts:146
   → POST http://127.0.0.1:8001/v1/chat
   → Body: {messages, model, stream: true, meta: {chat_id, chat_mode}}
   ↓
5. response.body.getReader() → ReadableStream<Uint8Array>
   ↓
6. consumeStream(reader, signal, onAbort, onChunk)
   → src/features/Chat/Thread/utils.ts:886
   → Decodes SSE format: "data: {json}\n\n"
   ↓
7. For each chunk: onChunk(json)
   → dispatch(chatResponse({...json, id: chatId}))
   ↓
8. Reducer: case chatResponse (reducer.ts:207)
   → formatChatResponse(state.thread.messages, payload)
   → Updates messages array immutably
   → Sets streaming: true, waiting_for_response: false
   ↓
9. ChatContent component re-renders with updated messages
   → Renders incrementally as content streams
   ↓
10. Stream ends: "data: [DONE]" or error
    → dispatch(doneStreaming({id: chatId}))
    → postProcessMessagesAfterStreaming()
    → streaming: false, read: true
```

### SSE Stream Format

**Protocol**: Server-Sent Events via ReadableStream

```
data: {"choices":[{"delta":{"role":"assistant","content":"Hello"},"finish_reason":null}]}\n\n
data: {"choices":[{"delta":{"content":" world"},"finish_reason":null}]}\n\n
data: {"choices":[{"delta":{},"finish_reason":"stop"}],"usage":{"total_tokens":50}}\n\n
data: [DONE]\n\n
```

**Special markers:**

- `data: [DONE]` - Stream complete
- `data: [ERROR]` - Generic error
- `data: {"detail":"..."}` - Structured error (LiteLLM format)
- `data: {"error":{"message":"..."}}` - LiteLLM streaming error

### The `consumeStream` Function

**Location**: `src/features/Chat/Thread/utils.ts:886`

**Key features:**

1. **Malformed chunk handling** - If buffer doesn't end with `\n\n`, combines with next chunk
2. **Error detection** - Checks for `{"detail":...}` at byte level before parsing
3. **Robust parsing** - Falls back to buffer combination on JSON parse errors
4. **Abort handling** - Respects AbortSignal for user cancellation

```typescript
export function consumeStream(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal: AbortSignal,
  onAbort: () => void,
  onChunk: (chunk: Record<string, unknown>) => void,
) {
  const decoder = new TextDecoder();

  function pump({
    done,
    value,
  }: ReadableStreamReadResult<Uint8Array>): Promise<void> {
    if (done) return Promise.resolve();
    if (signal.aborted) {
      onAbort();
      return Promise.resolve();
    }

    // Decode bytes to string
    const streamAsString = decoder.decode(value);

    // Split by SSE delimiter
    const deltas = streamAsString.split("\n\n").filter((str) => str.length > 0);

    for (const delta of deltas) {
      if (!delta.startsWith("data: ")) continue;

      const maybeJsonString = delta.substring(6); // Remove "data: "

      if (maybeJsonString === "[DONE]") return Promise.resolve();
      if (maybeJsonString === "[ERROR]")
        return Promise.reject(new Error("error from lsp"));

      // Parse JSON
      const json = parseOrElse<Record<string, unknown>>(maybeJsonString, {});
      onChunk(json);
    }

    return reader.read().then(pump); // Recursive read
  }

  return reader.read().then(pump);
}
```

### The `formatChatResponse` Function

**Location**: `src/features/Chat/Thread/utils.ts:331-650` (320 lines!)

**Purpose**: Merge streaming delta into existing messages array

**Response Types Handled:**

| Type                   | Detection              | Action                                       |
| ---------------------- | ---------------------- | -------------------------------------------- |
| `UserResponse`         | `role: "user"`         | Replace last user message (compression hint) |
| `ContextFileResponse`  | `role: "context_file"` | Append context files                         |
| `SubchatResponse`      | Has `subchat_id`       | Update tool call with subchat ID             |
| `ToolResponse`         | `role: "tool"`         | Append tool result message                   |
| `DiffResponse`         | `role: "diff"`         | Append diff chunks                           |
| `PlainTextResponse`    | `role: "plain_text"`   | Append plain text message                    |
| `SystemResponse`       | `role: "system"`       | **Prepend** to messages (goes first)         |
| **ChatResponseChoice** | Has `choices[]`        | **Merge delta into assistant message** ⭐    |

**Delta Types (in `choices[0].delta`):**

```typescript
delta: {
  role?: "assistant",
  content?: string,                    // Main response text
  reasoning_content?: string,          // Separate reasoning field
  tool_calls?: ToolCall[],             // Function calls
  thinking_blocks?: ThinkingBlock[],   // COT blocks
  provider_specific_fields?: {
    citation?: WebSearchCitation       // Web search results
  }
}
```

**Merging Logic:**

1. **Content delta** - Concatenate strings: `prevContent + delta.content`
2. **Tool calls delta** - `mergeToolCalls(prev, add)`:
   - If new tool (has `function.name`), append
   - If continuation (only `arguments`), concat to last tool's arguments
   - Handle missing IDs (generate UUID)
   - Handle broken indexes (Qwen3/sglang quirks)
3. **Thinking blocks** - `mergeThinkingBlocks(prev, add)`:
   - Always merge into first block
   - Concat `thinking` and `signature` strings
4. **Citations** - Append to array (web search links)
5. **Usage/Metering** - Take highest values (later chunks have final counts)

**Post-processing** (`postProcessMessagesAfterStreaming`):

- Deduplicate tool calls
- Filter out server-executed tools (`srvtoolu_*` prefix)
- Clean up incomplete tool calls

### State Transitions

```typescript
// Initial state
{
  streaming: false,
  waiting_for_response: false,
  prevent_send: false,
  thread: { messages: [] }
}

// After submit
dispatch(chatAskedQuestion) →
{
  waiting_for_response: true,  // Blocks duplicate sends
  prevent_send: false
}

// First chunk arrives
dispatch(chatResponse) →
{
  streaming: true,              // UI shows streaming indicator
  waiting_for_response: false,
  thread: { messages: [{role: "assistant", content: "H"}] }
}

// More chunks
dispatch(chatResponse) x N →
{
  streaming: true,
  thread: { messages: [{role: "assistant", content: "Hello world..."}] }
}

// Stream completes
dispatch(doneStreaming) →
{
  streaming: false,
  waiting_for_response: false,
  prevent_send: false,         // Allow next message
  thread: { read: true, messages: [...] }  // Mark as read
}

// Error
dispatch(chatError) →
{
  streaming: false,
  waiting_for_response: false,
  prevent_send: true,          // Block sends until error cleared
  error: "Error message"
}
```

### Tool Loop Detection

**Problem**: AI might call same tool repeatedly with same args (infinite loop)

**Solution**: `checkForToolLoop(messages)` (actions.ts:293)

- Scans recent assistant+tool messages
- Detects duplicate tool calls with identical results
- Sets `only_deterministic_messages: true` to stop streaming

### Queued Messages (Priority System)

**Feature**: User can send multiple messages while streaming

```typescript
type QueuedUserMessage = {
  id: string;
  message: UserMessage;
  createdAt: number;
  priority?: boolean; // Send immediately after current stream ends
};

// Regular queue: waits for tools to complete
// Priority queue: sends right after streaming (next turn)
```

**Hook**: `useAutoSend()` in `useSendChatRequest.ts:362-477`

- Monitors `queuedMessages`, `streaming`, `hasUnsentTools`
- Auto-flushes when appropriate conditions met
- Priority messages bypass tool completion wait

---

## State Management

### Redux Architecture

**Modern Redux Toolkit** with `combineSlices` (not legacy `combineReducers`)

**Store Setup**: `src/app/store.ts`

```typescript
import { combineSlices, configureStore } from "@reduxjs/toolkit";
import { listenerMiddleware } from "./middleware";

// Feature slices
import { chatSlice } from "../features/Chat/Thread/reducer";
import { historySlice } from "../features/History/historySlice";
import { configSlice } from "../features/Config/configSlice";
import { pagesSlice } from "../features/Pages/pagesSlice";
// ... 20+ more slices

// RTK Query APIs
import { capsApi } from "../services/refact/caps";
import { commandsApi } from "../services/refact/commands";
// ... 15+ more APIs

const rootReducer = combineSlices(
  chatSlice,
  historySlice,
  configSlice,
  // Auto-registers RTK Query reducers
  capsApi,
  commandsApi,
  // ...
);

export const store = configureStore({
  reducer: rootReducer,
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware()
      .prepend(listenerMiddleware.middleware)
      .concat(capsApi.middleware, commandsApi.middleware /* ... */),
});
```

### Key Slices

| Slice            | Purpose                | Location                                         | State Keys                                                                                |
| ---------------- | ---------------------- | ------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| **chat**         | Active thread + cache  | `features/Chat/Thread/reducer.ts`                | `thread`, `streaming`, `waiting_for_response`, `prevent_send`, `cache`, `queued_messages` |
| **history**      | Chat history (max 100) | `features/History/historySlice.ts`               | `chats`, `selectedId`                                                                     |
| **config**       | Global settings        | `features/Config/configSlice.ts`                 | `host`, `lspPort`, `apiKey`, `features`, `themeProps`                                     |
| **pages**        | Navigation stack       | `features/Pages/pagesSlice.ts`                   | `pages` (array of page objects)                                                           |
| **activeFile**   | IDE context            | `features/Chat/activeFile.ts`                    | `file_name`, `can_paste`, `cursor`                                                        |
| **checkpoints**  | Rollback UI state      | `features/Checkpoints/checkpointsSlice.ts`       | `previewData`, `restoreInProgress`                                                        |
| **confirmation** | Tool pause reasons     | `features/ToolConfirmation/confirmationSlice.ts` | `pauseReasons`, `wasInteracted`, `confirmationStatus`                                     |
| **errors**       | Error messages         | `features/Errors/errorsSlice.ts`                 | `errors` (array)                                                                          |
| **teams**        | Active team/group      | `features/Teams/teamsSlice.ts`                   | `activeGroup`                                                                             |

### RTK Query APIs

**All APIs** auto-generate hooks like `useGetCapsQuery`, `useUpdateModelMutation`

| API                 | Base URL                    | Purpose            | Key Endpoints                        |
| ------------------- | --------------------------- | ------------------ | ------------------------------------ |
| **capsApi**         | `/v1/caps`                  | Model capabilities | `getCaps`                            |
| **commandsApi**     | `/v1/at-command-completion` | Autocomplete       | `getCompletion`, `getPreview`        |
| **toolsApi**        | `/v1/tools`                 | Tool system        | `getTools`, `checkForConfirmation`   |
| **dockerApi**       | `/v1/docker-*`              | Container mgmt     | `getContainers`, `executeAction`     |
| **integrationsApi** | `/v1/integrations`          | Config files       | `getData`, `saveData`                |
| **modelsApi**       | `/v1/customization`         | Model config       | `getModels`, `updateModel`           |
| **providersApi**    | `/v1/customization`         | Provider config    | `getProviders`, `updateProvider`     |
| **checkpointsApi**  | `/v1/*_checkpoints`         | Workspace rollback | `preview`, `restore`                 |
| **pathApi**         | `/v1/*_path`                | File paths         | `getFullPath`, `customizationPath`   |
| **telemetryApi**    | `/v1/telemetry`             | Analytics          | `sendChatEvent`, `sendNetEvent`      |
| **linksApi**        | `/v1/links`                 | Smart links        | `getLinks`                           |
| **smallCloudApi**   | `https://www.smallcloud.ai` | Auth/user          | `getUser`, `getUserSurvey` (GraphQL) |

**Note**: Chat is NOT an RTK Query API - uses manual `fetch` with custom streaming logic.

### Selectors Pattern

**Always use selectors** (don't access `state.chat.thread.messages` directly)

```typescript
// src/features/Chat/Thread/selectors.ts

export const selectThread = (state: RootState) => state.chat.thread;
export const selectMessages = (state: RootState) => state.chat.thread.messages;
export const selectIsStreaming = (state: RootState) => state.chat.streaming;
export const selectChatId = (state: RootState) => state.chat.thread.id;

// Memoized selectors with Reselect
export const selectLastAssistantMessage = createSelector(
  [selectMessages],
  (messages) => {
    for (let i = messages.length - 1; i >= 0; i--) {
      if (isAssistantMessage(messages[i])) return messages[i];
    }
    return null;
  },
);

// Complex selectors
export const selectHasUncalledTools = createSelector(
  [selectMessages],
  (messages) => {
    const lastMsg = messages[messages.length - 1];
    if (!isAssistantMessage(lastMsg)) return false;
    if (!lastMsg.tool_calls) return false;
    return lastMsg.tool_calls.some((tc) => !isServerExecutedTool(tc.id));
  },
);
```

**30+ selectors** in `selectors.ts` - use them for consistency!

### Redux Persist

**Location**: `src/app/storage.ts`

```typescript
import { persistReducer } from "redux-persist";
import storage from "redux-persist/lib/storage"; // localStorage

const persistConfig = {
  key: "refact-chat",
  storage,
  whitelist: ["history", "config"], // Only persist these slices
  transforms: [pruneHistoryTransform], // Limit to 100 chats
};

// Prune old chats on save
const pruneHistoryTransform = createTransform(
  (inboundState: HistoryState) => {
    if (inboundState.chats.length <= 100) return inboundState;
    return {
      ...inboundState,
      chats: inboundState.chats.slice(-100), // Keep last 100
    };
  },
  null,
  { whitelist: ["history"] },
);
```

**Why only history + config?**

- Active chat (`state.chat`) is ephemeral
- Cache is cleared on app restart
- Prevents localStorage quota issues

### Middleware & Listeners

**Location**: `src/app/middleware.ts`

**Purpose**: Cross-cutting concerns that don't fit in reducers

```typescript
export const listenerMiddleware = createListenerMiddleware()

// 1. Error handling for RTK Query
listenerMiddleware.startListening({
  matcher: isAnyOf(
    capsApi.endpoints.getCaps.matchRejected,
    // ... other rejected matchers
  ),
  effect: (action, listenerApi) => {
    listenerApi.dispatch(addError({
      message: action.error.message,
      type: 'GLOBAL'
    }))
  }
})

// 2. IDE tool response handling
listenerMiddleware.startListening({
  actionCreator: ideToolCallResponse,
  effect: (action, listenerApi) => {
    const { toolCallId, chatId, accepted } = action.payload

    // Update history
    listenerApi.dispatch(upsertToolCallIntoHistory({...}))

    // Update active thread
    listenerApi.dispatch(upsertToolCall({...}))

    // Remove pause reason for this tool
    listenerApi.dispatch(updateConfirmationAfterIdeToolUse({...}))

    // Continue chat if no more pause reasons
    const state = listenerApi.getState()
    if (state.confirmation.pauseReasons.length === 0 && accepted) {
      listenerApi.dispatch(sendCurrentChatToLspAfterToolCallUpdate({
        chatId, toolCallId
      }))
    }
  }
})

// 3. Theme class updates
listenerMiddleware.startListening({
  predicate: (action, currentState, previousState) => {
    return currentState.config.themeProps?.appearance !==
           previousState.config.themeProps?.appearance
  },
  effect: (action, listenerApi) => {
    const appearance = listenerApi.getState().config.themeProps?.appearance
    document.body.className = appearance === 'light' ? 'vscode-light' : 'vscode-dark'
  }
})

// 10+ more listeners for:
// - Telemetry events
// - History auto-save
// - File reload triggers
// - JetBrains-specific tree refresh
```

**Key Pattern**: Use listeners for:

- Side effects (postMessage, telemetry)
- Cross-slice coordination
- Reacting to RTK Query lifecycle

---

## Component Hierarchy & Rendering

### Visual Component Tree

```
App (features/App.tsx)
├─ Provider Stack
│  ├─ Redux Provider
│  ├─ urql Provider (GraphQL)
│  ├─ PersistGate (redux-persist)
│  ├─ Theme (Radix)
│  ├─ TourProvider
│  └─ AbortControllerProvider
│
└─ InnerApp
   ├─ Sidebar (navigation)
   ├─ Toolbar (tabs if tabbed mode)
   │
   └─ PageWrapper (current page)
      ├─ Chat (main chat page) ⭐
      │  ├─ ChatHistory
      │  ├─ ChatContent ⭐⭐ (message renderer)
      │  │  ├─ UserInput (editable messages)
      │  │  ├─ AssistantInput (AI responses)
      │  │  │  ├─ ReasoningContent (thinking blocks)
      │  │  │  ├─ Markdown (main content)
      │  │  │  ├─ ToolsContent ⭐⭐⭐ (most complex)
      │  │  │  └─ Citations (web search links)
      │  │  ├─ DiffContent (file changes)
      │  │  ├─ QueuedMessage (pending sends)
      │  │  └─ SystemInput (system messages)
      │  │
      │  └─ ChatForm (input + controls)
      │     ├─ TextArea
      │     ├─ PromptSelect
      │     ├─ ToolConfirmation (pause popup)
      │     ├─ FilesPreview
      │     └─ AgentCapabilities
      │
      ├─ ThreadHistory (view old thread)
      ├─ Statistics (usage charts)
      ├─ Integrations (config UI)
      ├─ Providers (LLM config)
      └─ FIMDebug (debug panel)
```

### Critical Component: ChatContent

**Location**: `src/components/ChatContent/ChatContent.tsx` (283 lines)

**Purpose**: Dispatcher that routes message types to specialized renderers

**Core Algorithm**:

```typescript
function renderMessages(
  messages: ChatMessages,
  onRetry: (index, question) => void,
  waiting: boolean,
  memo: React.ReactNode[] = [],
  index = 0
): React.ReactNode[] {
  if (messages.length === 0) return memo

  const [head, ...tail] = messages

  // Route by message type
  if (head.role === 'tool') {
    return renderMessages(tail, onRetry, waiting, memo, index + 1)  // Skip tools
  }

  if (head.role === 'user') {
    return renderMessages(tail, onRetry, waiting,
      memo.concat(<UserInput key={index} message={head} index={index} />),
      index + 1
    )
  }

  if (head.role === 'assistant') {
    // Group consecutive diffs + tools with this assistant message
    const [diffMessages, toolMessages, rest] = groupRelatedMessages(tail)

    return renderMessages(rest, onRetry, waiting,
      memo.concat(
        <AssistantInput
          key={index}
          message={head}
          diffMessages={diffMessages}
          toolMessages={toolMessages}
          waiting={waiting}
        />
      ),
      index + diffMessages.length + toolMessages.length + 1
    )
  }

  // ... handle other types
  return renderMessages(tail, onRetry, waiting, memo, index + 1)
}
```

**Key Behavior**:

- **Recursive** processing (not `map`)
- **Groups** diffs + tools with assistant messages
- **Skips** tool messages (shown inline in AssistantInput)
- **Appends** memo (pure functional, no mutations)

### UserInput Component

**Props**:

```typescript
interface UserInputProps {
  message: UserMessage;
  index: number;
  onRetry?: (index: number, content: string) => void;
}
```

**Features**:

- **Editable** via inline textarea (click to edit)
- **Checkpoints** badge (if message has checkpoints)
- **Image attachments** (multi-modal content parsing)
- **Compression hint** 🗜️ icon
- **Context files** 🗃️ icon (memories)

**Content Types**:

```typescript
type UserMessage = {
  role: "user";
  content: string | UserMessageContent[]; // String or multi-modal
  checkpoints?: Checkpoint[];
  compression_strength?: "absent" | "weak" | "strong";
};

type UserMessageContent =
  | { type: "text"; text: string }
  | { type: "image_url"; image_url: { url: string } };
```

### AssistantInput Component

**Props**:

```typescript
interface AssistantInputProps {
  message: AssistantMessage;
  diffMessages: DiffMessage[];
  toolMessages: ToolMessage[];
  waiting: boolean;
  onRetry?: () => void;
}
```

**Rendering Order**:

1. **ReasoningContent** (thinking blocks) - collapsible
2. **Main content** (Markdown) - with syntax highlighting
3. **ToolsContent** (for each tool_call) - complex nested tree
4. **DiffContent** (grouped diffs) - apply/reject UI
5. **Citations** (web search results) - clickable links
6. **Like/Resend buttons** (bottom actions)
7. **Usage info** (tokens, cost) - footer

**Streaming Behavior**:

- Shows streaming indicator while `waiting || content.endsWith('▍')`
- Markdown renders incrementally (no flicker)
- Tool calls appear as they arrive

### ToolsContent Component ⭐

**Location**: `src/components/ChatContent/ToolsContent.tsx` (668 lines!)

**Why so complex?**

- Handles 10+ tool types
- Nested subchats (5 levels deep possible)
- Multi-modal results (text, images, files)
- Special cases: Knowledge, TextDoc browser

**Visual Structure**:

```
ToolsContent (one per tool_call)
├─ Header (tool name, status badge)
├─ Arguments (collapsible JSON)
│
└─ Result (polymorphic by tool type)
   ├─ TextResult (most tools)
   ├─ KnowledgeResults (search results with scores)
   │  └─ FileList (clickable files)
   ├─ TextDocContent (file browser)
   │  ├─ FileTree navigation
   │  ├─ File content viewer
   │  └─ SmartLinks (context actions)
   └─ MultiModalResult (images + text)
      └─ DialogImage (lightbox)
```

**Tool Status Badge**:

- ⏳ `thinking` - Tool executing
- ✅ `success` - Completed
- ❌ `error` - Failed
- ☁️ `server` - Server-executed tool (display only)

**Special Tool Types**:

| Tool Type            | Component           | Notes                                      |
| -------------------- | ------------------- | ------------------------------------------ |
| `knowledge`          | KnowledgeResults    | Shows search results with relevance scores |
| `textdoc`            | TextDocContent      | Interactive file browser with navigation   |
| `subchat_*`          | Nested ToolsContent | Recursive subchat rendering (max 5 deep)   |
| `patch`, `text_edit` | DiffContent         | Shows in DiffContent, not ToolsContent     |
| Server tools         | Badge only          | `srvtoolu_*` prefix, no execution UI       |

### DiffContent Component

**Location**: `src/components/ChatContent/DiffContent.tsx` (364 lines)

**Purpose**: Group and display file changes with apply/reject controls

**Grouping Logic**:

```typescript
// Groups consecutive diffs by tool_call_id
const groupedDiffs = diffMessages.reduce<GroupedDiffs>((acc, msg) => {
  const key = msg.tool_call_id || "ungrouped";
  if (!acc[key]) acc[key] = [];
  acc[key].push(msg);
  return acc;
}, {});
```

**Each Group Renders**:

- **Header**: Tool name, file count, timestamps
- **Diff Viewer**: Line-by-line changes with syntax highlighting
- **Actions**: Apply All, Reject All (per group)
- **IDE Link**: Clickable file paths (opens in IDE)

**Diff Format**:

```typescript
type DiffChunk = {
  file_name: string;
  file_action: "A" | "M" | "D"; // Added/Modified/Deleted
  line1: number;
  line2: number;
  chunks: string; // Unified diff format
};
```

### Message Type Routing Summary

| Role           | Component                  | Skip Render? | Group With?   |
| -------------- | -------------------------- | ------------ | ------------- |
| `user`         | UserInput                  | No           | -             |
| `assistant`    | AssistantInput             | No           | diffs + tools |
| `tool`         | (inline in AssistantInput) | Yes          | -             |
| `diff`         | DiffContent                | No (grouped) | assistant     |
| `context_file` | ContextFiles               | No           | -             |
| `system`       | SystemInput                | No           | -             |
| `plain_text`   | PlainText                  | No           | -             |

### Special Content Markers

**In UI, look for these icons**:

| Icon | Meaning                              | Location         |
| ---- | ------------------------------------ | ---------------- |
| 🗜️   | Compression hint (context too large) | UserInput        |
| 🗃️   | Memory/context files attached        | UserInput        |
| ⏳   | Tool thinking                        | ToolsContent     |
| ✅   | Tool success                         | ToolsContent     |
| ❌   | Tool failed                          | ToolsContent     |
| ☁️   | Server-executed tool                 | ToolsContent     |
| 🔄   | Checkpoint reset available           | CheckpointButton |

---

## UI & Styling

### Styling Architecture

**Two-layer system**: **Radix UI Themes** + **CSS Modules**

```
Radix Themes (design tokens)
     ↓ provides
CSS Variables (--space-*, --color-*, --radius-*)
     ↓ used by
CSS Modules (component-specific styles)
```

### Golden Rules

1. ✅ **Use Radix primitives for layout**: `Flex`, `Box`, `Text`, `Card`, `Button`
2. ✅ **Use design tokens** (not magic numbers): `var(--space-3)`, `var(--color-accent-9)`
3. ✅ **CSS Modules** for component-specific styles: `styles.chatContent`
4. ❌ **Avoid global CSS** (exception: `src/lib/render/web.css` for body baseline)
5. ❌ **No inline styles** (use CSS Modules or Radix props)
6. ❌ **No magic numbers** (`padding: 8px` → `padding: var(--space-2)`)

### Radix Design Tokens

**Spacing** (based on 4px grid):

```css
--space-1: 4px --space-2: 8px --space-3: 12px --space-4: 16px --space-5: 20px
  --space-6: 24px --space-7: 28px --space-8: 32px --space-9: 36px;
```

**Colors** (semantic tokens):

```css
--accent-1 through --accent-12  /* Primary brand color scale */
--gray-1 through --gray-12      /* Neutral grays */
--color-background              /* Page background */
--color-surface                 /* Card background */
--color-panel-solid             /* Overlay background */
```

**Radii**:

```css
--radius-1: 4px --radius-2: 6px --radius-3: 8px --radius-4: 12px --radius-full:
  9999px;
```

**Typography**:

```css
--font-size-1 through --font-size-9
--line-height-1 through --line-height-9
--font-weight-regular: 400
--font-weight-medium: 500
--font-weight-bold: 700
```

### Theme Configuration

**Component**: `src/components/Theme/Theme.tsx`

```typescript
interface ThemeProps {
  appearance?: 'light' | 'dark' | 'inherit'
  accentColor?: 'indigo' | 'blue' | 'green' | /* ... */
  grayColor?: 'gray' | 'mauve' | 'slate' | 'auto'
  radius?: 'none' | 'small' | 'medium' | 'large' | 'full'
  scaling?: '90%' | '95%' | '100%' | '105%' | '110%'
}

export function Theme({ children }: { children: React.ReactNode }) {
  const config = useConfig()
  const appearance = useAppearance()  // Listens to OS/IDE theme

  return (
    <RadixTheme
      appearance={appearance}
      accentColor={config.themeProps?.accentColor ?? 'indigo'}
      grayColor={config.themeProps?.grayColor ?? 'auto'}
      radius={config.themeProps?.radius ?? 'medium'}
      scaling={config.themeProps?.scaling ?? '100%'}
    >
      {children}
    </RadixTheme>
  )
}
```

**Host-specific behavior**:

- `host === 'web'`: Wrapper includes dev theme toggle
- `host === 'vscode' | 'jetbrains'`: No wrapper, IDE controls theme
- `document.body.className`: Set to `vscode-light` or `vscode-dark` by middleware

### CSS Modules Pattern

**File naming**: `Component.module.css`

**Example** (`ChatContent.module.css`):

```css
.scroll_area {
  height: 100%;
  padding: var(--space-2) var(--space-4);
}

.message_group {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.streaming_indicator {
  color: var(--accent-9);
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}
```

**Usage in component**:

```typescript
import styles from './ChatContent.module.css'

export function ChatContent() {
  return (
    <div className={styles.scroll_area}>
      <div className={styles.message_group}>
        {/* ... */}
      </div>
    </div>
  )
}
```

**Conditional classes**:

```typescript
import classNames from 'classnames'

<div className={classNames(
  styles.message,
  isStreaming && styles.streaming,
  hasError && styles.error
)} />
```

### Common Patterns

**Layout with Radix**:

```typescript
<Flex direction="column" gap="3" p="4">
  <Box>Header</Box>
  <Box flexGrow="1">Content</Box>
  <Box>Footer</Box>
</Flex>
```

**Typography**:

```typescript
<Text size="2" weight="medium" color="gray">
  Label text
</Text>
```

**Cards**:

```typescript
<Card size="2" variant="surface">
  <Flex direction="column" gap="2">
    {/* content */}
  </Flex>
</Card>
```

**Buttons**:

```typescript
<Button size="2" variant="soft" onClick={handleClick}>
  Action
</Button>
```

### Responsive Design

**Minimal responsive styling** (app is designed for IDE sidebars)

**Breakpoints** (when needed):

```css
@media (max-width: 768px) {
  .sidebar {
    display: none;
  }
}
```

**Flex-based layout** handles most responsive needs automatically.

### Dark/Light Mode

**How it works**:

1. User/OS sets `appearance: 'light' | 'dark'`
2. Radix Theme applies appropriate color scales
3. All Radix tokens update automatically
4. Custom CSS uses tokens, so it updates too

**Testing dark mode**:

- Web: Use theme toggle in UI
- VSCode: Change VSCode theme
- JetBrains: Change IDE theme

**Custom dark mode overrides** (rare):

```css
.my_component {
  background: var(--color-surface);
}

/* Only if Radix token doesn't work */
:is(.dark, .dark-theme) .my_component {
  background: #1a1a1a;
}
```

### Icons

**Radix Icons**:

```typescript
import { ChevronDownIcon, CheckIcon, Cross2Icon } from '@radix-ui/react-icons'

<ChevronDownIcon width={16} height={16} />
```

**Custom icons** (rare):

```typescript
// src/images/
export function CustomIcon() {
  return <svg>{/* ... */}</svg>
}
```

### Animations

**Framer Motion** for complex animations:

```typescript
import { motion } from 'framer-motion'

<motion.div
  initial={{ opacity: 0, y: -10 }}
  animate={{ opacity: 1, y: 0 }}
  exit={{ opacity: 0, y: 10 }}
>
  {content}
</motion.div>
```

**CSS animations** for simple effects:

```css
@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.fade_in {
  animation: fadeIn 0.2s ease-in-out;
}
```

### Common Mistakes to Avoid

❌ **Using px values directly**:

```css
/* Bad */
.button {
  padding: 12px;
}

/* Good */
.button {
  padding: var(--space-3);
}
```

❌ **Hardcoded colors**:

```css
/* Bad */
.text {
  color: #3b82f6;
}

/* Good */
.text {
  color: var(--accent-9);
}
```

❌ **Global styles without scoping**:

```css
/* Bad - affects everything */
button {
  border-radius: 8px;
}

/* Good - scoped to module */
.my_button {
  border-radius: var(--radius-3);
}
```

❌ **Ignoring Radix primitives**:

```tsx
/* Bad - reinventing the wheel */
<div style={{display: 'flex', gap: '12px'}}>

/* Good - use Radix */
<Flex gap="3">
```

---

## API Services

### Service Architecture

**Two separate backends**:

```
┌─────────────────────────────────────────┐
│         Frontend (React)                │
├─────────────────────────────────────────┤
│  RTK Query APIs                         │
│  - capsApi, toolsApi, dockerApi, etc.   │
└────┬──────────────────────────┬─────────┘
     │                          │
     ▼                          ▼
┌─────────────────┐    ┌─────────────────┐
│  Local LSP      │    │  SmallCloud.ai  │
│  127.0.0.1:8001 │    │  (cloud)        │
│                 │    │                 │
│  - Chat         │    │  - Auth         │
│  - Tools        │    │  - User mgmt    │
│  - Caps         │    │  - Teams        │
│  - Models       │    │  - Surveys      │
│  - Docker       │    │                 │
│  - Integrations │    │  (GraphQL)      │
└─────────────────┘    └─────────────────┘
```

**Critical distinction**:

- **Chat ALWAYS goes to LSP** (never SmallCloud)
- LSP handles all AI operations
- SmallCloud only for auth/user/team management

### LSP Server Endpoints

**Base URL**: `http://127.0.0.1:${lspPort}/v1/...`

| Endpoint                       | Method | Purpose               | RTK Query API                   |
| ------------------------------ | ------ | --------------------- | ------------------------------- |
| `/v1/chat`                     | POST   | **Streaming chat**    | ❌ Manual fetch                 |
| `/v1/caps`                     | GET    | Model capabilities    | `capsApi.getCaps`               |
| `/v1/at-command-completion`    | POST   | Autocomplete          | `commandsApi.getCompletion`     |
| `/v1/at-command-preview`       | POST   | Preview command       | `commandsApi.getPreview`        |
| `/v1/tools`                    | POST   | Get available tools   | `toolsApi.getTools`             |
| `/v1/tools/check_confirmation` | POST   | Check tool approval   | `toolsApi.checkForConfirmation` |
| `/v1/docker-container-list`    | POST   | List containers       | `dockerApi.getContainers`       |
| `/v1/docker-container-action`  | POST   | Execute action        | `dockerApi.executeAction`       |
| `/v1/integrations-list`        | GET    | List integrations     | `integrationsApi.getList`       |
| `/v1/integration-get`          | POST   | Get config            | `integrationsApi.getData`       |
| `/v1/integration-save`         | POST   | Save config           | `integrationsApi.saveData`      |
| `/v1/preview_checkpoints`      | POST   | Preview rollback      | `checkpointsApi.preview`        |
| `/v1/restore_checkpoints`      | POST   | Apply rollback        | `checkpointsApi.restore`        |
| `/v1/get_file_text`            | POST   | Read file             | `pathApi.getFileText`           |
| `/v1/*_path`                   | GET    | Get config paths      | `pathApi.*Path`                 |
| `/v1/customization`            | POST   | Model/provider config | `modelsApi`, `providersApi`     |
| `/v1/telemetry/chat`           | POST   | Send telemetry        | `telemetryApi.sendChatEvent`    |
| `/v1/ping`                     | GET    | Health check          | `pingApi.getPing`               |

### RTK Query API Pattern

**All APIs follow this structure**:

```typescript
// src/services/refact/caps.ts
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export const capsApi = createApi({
  reducerPath: "caps",
  baseQuery: fetchBaseQuery({
    baseUrl: (_, api) => {
      const state = api.getState() as RootState;
      return `http://127.0.0.1:${state.config.lspPort}`;
    },
    prepareHeaders: (headers, { getState }) => {
      const state = getState() as RootState;
      if (state.config.apiKey) {
        headers.set("Authorization", `Bearer ${state.config.apiKey}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getCaps: builder.query<CapsResponse, void>({
      query: () => "/v1/caps",
    }),
  }),
});

export const { useGetCapsQuery, useLazyGetCapsQuery } = capsApi;
```

**Key features**:

- **Dynamic base URL** from Redux state
- **Auto-injects auth** token if present
- **Auto-generates hooks**: `useGetCapsQuery`, `useLazyGetCapsQuery`
- **Caching** by default

### Chat API (Special Case)

**Why not RTK Query?** Streaming + custom chunking logic

**Location**: `src/services/refact/chat.ts`

```typescript
export async function sendChat({
  messages,
  model,
  stream: true,
  abortSignal,
  chatId,
  port = 8001,
  apiKey,
  mode,
  // ...
}: SendChatArgs): Promise<Response> {
  const body = JSON.stringify({
    messages,
    model,
    stream: true,
    meta: {
      chat_id: chatId,
      chat_mode: mode ?? 'EXPLORE',
      // ...
    }
  })

  const headers = {
    'Content-Type': 'application/json',
    ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {})
  }

  const url = `http://127.0.0.1:${port}/v1/chat`

  return fetch(url, {
    method: 'POST',
    headers,
    body,
    signal: abortSignal,
    credentials: 'same-origin'
  })
}
```

**Response format** (SSE):

```
data: {"choices":[{"delta":{"role":"assistant","content":"Hi"},...}]}\n\n
data: {"choices":[{"delta":{"content":" there"},...}]}\n\n
data: [DONE]\n\n
```

### SmallCloud API (GraphQL)

**Base URL**: `https://www.smallcloud.ai/v1/graphql`

**Used for**:

- User authentication (OAuth)
- User profile
- Team management
- Usage surveys

**Setup**: `urqlProvider.tsx`

```typescript
const client = createClient({
  url: "https://www.smallcloud.ai/v1/graphql",
  fetchOptions: () => {
    const apiKey = store.getState().config.apiKey;
    return {
      headers: {
        ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
      },
    };
  },
  exchanges: [cacheExchange, fetchExchange, subscriptionExchange],
});
```

**Example queries** (generated from GraphQL schema):

```typescript
// useGetUser hook
const [result] = useQuery({
  query: graphql(`
    query GetUser {
      user {
        account
        email
        has_valid_subscription
      }
    }
  `),
});
```

**Note**: GraphQL codegen runs via `npm run generate:graphql`

### Type Definitions

**All API types** in `src/services/refact/types.ts` (787 lines!)

**Key types**:

```typescript
// Message types
export type UserMessage = {
  role: 'user'
  content: string | UserMessageContent[]
  checkpoints?: Checkpoint[]
  compression_strength?: 'absent' | 'weak' | 'strong'
}

export type AssistantMessage = {
  role: 'assistant'
  content: string
  reasoning_content?: string
  tool_calls?: ToolCall[]
  thinking_blocks?: ThinkingBlock[]
  citations?: WebSearchCitation[]
  finish_reason?: 'stop' | 'length' | 'tool_calls' | null
  usage?: Usage
  // Metering fields
  metering_balance?: number
  metering_*_tokens_n?: number
  metering_coins_*?: number
}

export type ToolCall = {
  id: string
  index: number
  function: {
    name: string
    arguments: string  // JSON string
  }
  subchat?: string  // Subchat ID if nested
  attached_files?: string[]  // Files attached to subchat
}

export type ToolMessage = {
  role: 'tool'
  content: ToolResult
}

export type ToolResult = {
  tool_call_id: string
  content: string | { type: 'image_url', image_url: { url: string } }[]
  finish_reason?: 'stop' | 'length' | null
  compression_strength?: 'absent' | 'weak' | 'strong'
  tool_failed?: boolean
}

// Diff types
export type DiffMessage = {
  role: 'diff'
  content: DiffChunk[]
  tool_call_id?: string
}

export type DiffChunk = {
  file_name: string
  file_action: 'A' | 'M' | 'D'
  line1: number
  line2: number
  chunks: string  // Unified diff
}

// Response types (streaming deltas)
export type ChatResponse =
  | ChatResponseChoice
  | UserResponse
  | ContextFileResponse
  | ToolResponse
  | DiffResponse
  | SubchatResponse
  | SystemResponse
  | PlainTextResponse
```

**Type guards** (critical for message routing):

```typescript
export function isUserMessage(msg: unknown): msg is UserMessage {
  return (
    typeof msg === "object" &&
    msg !== null &&
    "role" in msg &&
    msg.role === "user"
  );
}

export function isAssistantMessage(msg: unknown): msg is AssistantMessage {
  return (
    typeof msg === "object" &&
    msg !== null &&
    "role" in msg &&
    msg.role === "assistant"
  );
}

// ... 20+ more type guards
```

### Error Handling

**RTK Query errors** are caught by middleware:

```typescript
listenerMiddleware.startListening({
  matcher: isAnyOf(
    capsApi.endpoints.getCaps.matchRejected,
    toolsApi.endpoints.getTools.matchRejected,
    // ...
  ),
  effect: (action, listenerApi) => {
    const error = action.error;
    listenerApi.dispatch(
      addError({
        message: error.message ?? "Unknown error",
        type: "GLOBAL",
      }),
    );
  },
});
```

**Chat errors** handled in thunk:

```typescript
.catch((err: unknown) => {
  dispatch(doneStreaming({ id: chatId }))
  dispatch(chatError({
    id: chatId,
    message: err instanceof Error ? err.message : String(err)
  }))
})
```

---

## IDE Integration

### postMessage Architecture

**Communication protocol** between GUI (iframe) and IDE extension (host)

```
┌─────────────────────────────────────────┐
│     IDE Extension (VSCode/JetBrains)    │
│                                         │
│  window.postMessage(event, '*')         │
└──────────────┬──────────────────────────┘
               │
               │ postMessage API
               │
┌──────────────▼──────────────────────────┐
│     GUI (React in iframe/webview)       │
│                                         │
│  window.addEventListener('message', ...) │
└─────────────────────────────────────────┘
```

### Message Flow Directions

**1. IDE → GUI** (context updates, responses)

Handled by: `src/hooks/useEventBusForApp.ts`

```typescript
export function useEventBusForApp() {
  const dispatch = useAppDispatch();

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      // File context update
      if (setFileInfo.match(event.data)) {
        dispatch(setFileInfo(event.data.payload));
      }

      // Selected code snippet
      if (setSelectedSnippet.match(event.data)) {
        dispatch(setSelectedSnippet(event.data.payload));
      }

      // New chat trigger
      if (newChatAction.match(event.data)) {
        if (!isPageInHistory({ pages }, "chat")) {
          dispatch(push({ name: "chat" }));
        }
        dispatch(newChatAction(event.data.payload));
      }

      // Tool approval response
      if (ideToolCallResponse.match(event.data)) {
        dispatch(event.data);
      }

      // ... more handlers
    };

    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, [dispatch]);
}
```

**2. GUI → IDE** (commands, requests)

Handled by: `src/hooks/useEventBusForIDE.ts`

```typescript
export const useEventsBusForIDE = () => {
  const postMessage = usePostMessage();

  const openFile = useCallback(
    (file: OpenFilePayload) => {
      const action = ideOpenFile(file);
      postMessage(action);
    },
    [postMessage],
  );

  const diffPasteBack = useCallback(
    (content: string, chatId?: string) => {
      const action = ideDiffPasteBackAction({ content, chatId });
      postMessage(action);
    },
    [postMessage],
  );

  const sendToolCallToIde = useCallback(
    (toolCall, edit, chatId) => {
      const action = ideToolCall({ toolCall, edit, chatId });
      postMessage(action);
    },
    [postMessage],
  );

  // ... 20+ command functions

  return {
    openFile,
    diffPasteBack,
    sendToolCallToIde,
    // ...
  };
};
```

### postMessage Transport

**Location**: `src/hooks/usePostMessage.ts`

**Auto-detects host**:

```typescript
export function usePostMessage() {
  const config = useConfig();

  return useCallback(
    (message: unknown) => {
      if (config.host === "vscode") {
        // VSCode uses acquireVsCodeApi
        const vscode = window.acquireVsCodeApi?.();
        vscode?.postMessage(message);
      } else if (config.host === "jetbrains") {
        // JetBrains uses custom function
        window.postIntellijMessage?.(message);
      } else {
        // Web/generic: use window.postMessage
        window.postMessage(message, "*");
      }
    },
    [config.host],
  );
}
```

### Event Types

**Defined in**: `src/events/setup.ts`, IDE action creators

**Common events IDE → GUI**:

| Event Type              | Payload                          | Purpose              |
| ----------------------- | -------------------------------- | -------------------- |
| `updateConfig`          | `Partial<Config>`                | Update global config |
| `setFileInfo`           | `{file_name, can_paste}`         | Active file changed  |
| `setSelectedSnippet`    | `{code, language}`               | Code selection       |
| `newChatAction`         | `Partial<ChatThread>`            | Start new chat       |
| `ideToolCallResponse`   | `{toolCallId, chatId, accepted}` | Tool approval        |
| `setCurrentProjectInfo` | `{name, path}`                   | Project context      |

**Common events GUI → IDE**:

| Event Type                  | Payload                    | Purpose                 |
| --------------------------- | -------------------------- | ----------------------- |
| `ideOpenFile`               | `{file_path, line?}`       | Open file in editor     |
| `ideDiffPasteBack`          | `{content, chatId}`        | Apply code changes      |
| `ideToolCall`               | `{toolCall, edit, chatId}` | Request tool execution  |
| `ideOpenSettings`           | -                          | Open settings UI        |
| `ideNewFile`                | `{content}`                | Create new file         |
| `ideAnimateFileStart/Stop`  | `{file_name}`              | File activity indicator |
| `ideChatPageChange`         | `{page}`                   | Navigation event        |
| `ideSetCodeCompletionModel` | `{model}`                  | Update model            |
| `ideSetActiveTeamsGroup`    | `{group}`                  | Set active team         |

### Host Mode Differences

**Config**: `state.config.host: 'web' | 'vscode' | 'jetbrains' | 'ide'`

| Feature                  | web                  | vscode               | jetbrains               | ide        |
| ------------------------ | -------------------- | -------------------- | ----------------------- | ---------- |
| **postMessage**          | `window.postMessage` | `acquireVsCodeApi()` | `postIntellijMessage()` | Generic    |
| **Theme**                | Toggle in UI         | VSCode controls      | JB controls             | Generic    |
| **File links**           | ❌ No-op             | ✅ Opens in editor   | ✅ Opens in IDE         | ✅ Generic |
| **Copy buttons**         | ✅ Visible           | ❌ Hidden            | ❌ Hidden               | ❌ Hidden  |
| **Tool execution**       | LSP only             | LSP + IDE            | LSP + IDE               | LSP + IDE  |
| **Paste to file**        | ❌ No-op             | ✅ Works             | ✅ Works                | ✅ Works   |
| **Project tree refresh** | N/A                  | N/A                  | ✅ Auto-refresh         | N/A        |

**Host detection**:

```typescript
const config = useConfig();
const isIDE = config.host !== "web";
const isVSCode = config.host === "vscode";
const isJetBrains = config.host === "jetbrains";
```

### Tool Approval Flow (IDE-specific)

**For patch-like tools**, IDE shows preview before applying:

```
1. AI suggests patch tool_call
   ↓
2. GUI: Confirmation popup (if not automatic_patch)
   ↓
3. User confirms
   ↓
4. GUI → IDE: ideToolCall({toolCall, edit, chatId})
   ↓
5. IDE: Shows diff preview
   ↓
6. User: Applies or rejects
   ↓
7. IDE → GUI: ideToolCallResponse({toolCallId, chatId, accepted})
   ↓
8. GUI middleware: Updates tool status, continues chat
```

**Web mode**: All tools executed by LSP directly (no IDE approval step)

---

## Tool Calling System

### Overview

The tool calling system allows AI to execute functions (file operations, shell commands, searches, etc.) with optional user confirmation.

### Tool Call Lifecycle

```
1. AI Response with tool_calls
   ↓
2. [Confirmation Gate] ← configurable
   ↓
3. Tool Execution (LSP or IDE)
   ↓
4. Tool Result inserted as message
   ↓
5. AI continues with result
   ↓
6. Loop until finish_reason: "stop"
```

### Confirmation Logic

**Location**: `src/hooks/useSendChatRequest.ts` (lines 138-201)

**Decision tree**:

```typescript
async function sendMessages(messages, maybeMode) {
  dispatch(setIsWaitingForResponse(true));
  const lastMessage = messages.slice(-1)[0];

  // Check if last message has tool_calls
  if (
    !isWaiting &&
    !wasInteracted &&
    isAssistantMessage(lastMessage) &&
    lastMessage.tool_calls
  ) {
    const toolCalls = lastMessage.tool_calls;

    // Check for automatic bypass
    if (
      toolCalls[0].function.name &&
      PATCH_LIKE_FUNCTIONS.includes(toolCalls[0].function.name) &&
      isPatchAutomatic // ← per-chat setting
    ) {
      // Skip confirmation for patch-like tools in automatic mode
    } else {
      // Ask backend if confirmation needed
      const confirmationResponse = await triggerCheckForConfirmation({
        tool_calls: toolCalls,
        messages: messages,
      }).unwrap();

      if (confirmationResponse.pause) {
        dispatch(setPauseReasons(confirmationResponse.pause_reasons));
        return; // STOP - show confirmation UI
      }
    }
  }

  // Proceed with LSP call
  dispatch(backUpMessages({ id: chatId, messages }));
  dispatch(chatAskedQuestion({ id: chatId }));
  // ... sendChat()
}
```

### PATCH_LIKE_FUNCTIONS

**These tools auto-approve when `automatic_patch === true`**:

```typescript
export const PATCH_LIKE_FUNCTIONS = [
  "patch",
  "text_edit",
  "create_textdoc",
  "update_textdoc",
  "replace_textdoc",
  "update_textdoc_regex",
  "update_textdoc_by_lines",
];
```

### Confirmation API

**Endpoint**: `POST /v1/tools/check_confirmation`

**Request**:

```json
{
  "tool_calls": [
    {
      "id": "call_123",
      "function": {
        "name": "patch",
        "arguments": "{\"file_path\":\"src/app.ts\",...}"
      }
    }
  ],
  "messages": [
    /* full context */
  ]
}
```

**Response**:

```json
{
  "pause": true,
  "pause_reasons": [
    {
      "type": "confirmation",
      "rule": "*.py files require approval",
      "tool_call_id": "call_123"
    }
  ]
}
```

**If `pause === false`**: Tool executes immediately  
**If `pause === true`**: Show ToolConfirmation popup

### ToolConfirmation Component

**Location**: `src/components/ChatForm/ToolConfirmation.tsx`

**UI shows**:

- **Tool name** (e.g., "patch")
- **Arguments** (collapsible JSON)
- **Pause reason** (e.g., "requires approval")
- **Three buttons**:
  - 🟢 **Allow Once** - Confirm this tool, continue
  - 🟢 **Allow Chat** - Enable automatic mode for this chat
  - 🔴 **Stop** - Reject tool, end chat

**User actions**:

```typescript
// Allow Once
const confirmToolUsage = () => {
  dispatch(
    clearPauseReasonsAndHandleToolsStatus({
      wasInteracted: true,
      confirmationStatus: true,
    }),
  );
  dispatch(setIsWaitingForResponse(false));
  // useAutoSend will detect clear and continue
};

// Allow Chat
const enableAutomaticPatch = () => {
  dispatch(setAutomaticPatch({ chatId, value: true }));
  confirmToolUsage();
};

// Stop
const rejectToolUsage = (toolCallIds) => {
  toolCallIds.forEach((id) => {
    dispatch(upsertToolCall({ toolCallId: id, chatId, accepted: false }));
  });
  dispatch(resetConfirmationInteractedState());
  dispatch(setIsWaitingForResponse(false));
  dispatch(doneStreaming({ id: chatId }));
  dispatch(setPreventSend({ id: chatId }));
};
```

### Tool Execution Paths

**Two execution models**:

#### 1. LSP-Executed Tools (Most tools)

```
GUI → LSP /v1/chat with tool_calls → LSP executes → Returns tool result
```

**Examples**: `shell`, `read_file`, `search`, `web_search`, etc.

**Result format**:

```json
{
  "role": "tool",
  "tool_call_id": "call_123",
  "content": "Command output...",
  "finish_reason": "stop"
}
```

#### 2. IDE-Executed Tools (Patch-like tools)

```
GUI → LSP /v1/chat with tool_calls
  ↓
LSP returns tool instruction (not executed yet)
  ↓
GUI → IDE: ideToolCall({toolCall, edit, chatId})
  ↓
IDE: Shows diff preview, user applies/rejects
  ↓
IDE → GUI: ideToolCallResponse({toolCallId, chatId, accepted})
  ↓
GUI: Inserts tool result, continues chat
```

**Edit format** (`ToolEditResult`):

```typescript
type ToolEditResult = {
  file_name: string;
  file_action: "A" | "M" | "D";
  line1: number;
  line2: number;
  chunks: string; // Unified diff
};
```

### Server-Executed Tools

**Special case**: Tools with `id.startsWith('srvtoolu_')`

**Behavior**:

- Already executed by LLM provider (e.g., Claude with computer use)
- GUI shows badge: ☁️ "Server tool"
- NOT sent to LSP for execution
- Display only (no confirmation needed)

**Detection**:

```typescript
export function isServerExecutedTool(toolCallId?: string): boolean {
  return toolCallId?.startsWith("srvtoolu_") ?? false;
}
```

### Tool Result Insertion

**Via IDE approval** (middleware listener):

```typescript
listenerMiddleware.startListening({
  actionCreator: ideToolCallResponse,
  effect: (action, listenerApi) => {
    const { toolCallId, chatId, accepted } = action.payload;

    // 1. Update history
    listenerApi.dispatch(
      upsertToolCallIntoHistory({
        toolCallId,
        chatId,
        accepted,
      }),
    );

    // 2. Insert/update tool result in messages
    listenerApi.dispatch(
      upsertToolCall({
        toolCallId,
        chatId,
        accepted,
      }),
    );

    // 3. Remove pause reason
    listenerApi.dispatch(
      updateConfirmationAfterIdeToolUse({
        toolCallId,
      }),
    );

    // 4. Continue chat if no more pauses
    const state = listenerApi.getState();
    if (state.confirmation.pauseReasons.length === 0 && accepted) {
      listenerApi.dispatch(
        sendCurrentChatToLspAfterToolCallUpdate({
          chatId,
          toolCallId,
        }),
      );
    }
  },
});
```

**Via streaming** (LSP returns tool message):

- Handled by `formatChatResponse` in reducer
- Tool message appended to `thread.messages`

### Tool Loop Prevention

**Problem**: AI might call same tool repeatedly (infinite loop)

**Solution**: `checkForToolLoop(messages)` in actions

```typescript
function checkForToolLoop(messages): boolean {
  // Get recent assistant+tool messages
  const recentMessages = takeFromEndWhile(messages, msg =>
    isToolMessage(msg) || isToolCallMessage(msg)
  )

  // Extract tool calls and results
  const toolCalls = /* ... */
  const toolResults = /* ... */

  // Check for duplicates (same tool, args, AND result)
  return scanForDuplicatesWith(toolCalls, (a, b) => {
    const aResult = toolResults.find(msg => msg.content.tool_call_id === a.id)
    const bResult = toolResults.find(msg => msg.content.tool_call_id === b.id)

    return (
      a.function.name === b.function.name &&
      a.function.arguments === b.function.arguments &&
      aResult?.content === bResult?.content
    )
  })
}
```

**If loop detected**:

- Sets `only_deterministic_messages: true` in LSP request
- Stops streaming to prevent infinite loop

### Subchat System

**Feature**: Tools can spawn nested chats

**Use case**: Multi-step research, recursive search

**Flow**:

```
Tool call → LSP creates subchat → Subchat executes → Files attached to parent tool
```

**Message format**:

```typescript
type SubchatResponse = {
  subchat_id: string;
  tool_call_id: string;
  add_message: ContextFileResponse;
};
```

**Rendering**: ToolsContent renders nested subchats recursively (max 5 deep)

### Tool Status States

```typescript
type ToolStatus =
  | "thinking" // ⏳ Executing
  | "success" // ✅ Completed
  | "error" // ❌ Failed
  | "server"; // ☁️ Server-executed (display only)
```

**Visual indicators** in ToolsContent component

### Common Tool Types

| Tool                        | Purpose        | Execution | Confirmation? |
| --------------------------- | -------------- | --------- | ------------- |
| `patch`                     | Edit files     | IDE       | Optional      |
| `text_edit`                 | Edit files     | IDE       | Optional      |
| `shell`                     | Run commands   | LSP       | Optional      |
| `read_file`                 | Read file      | LSP       | Rare          |
| `search`                    | Code search    | LSP       | No            |
| `web_search`                | Search web     | LSP       | No            |
| `knowledge`                 | Vec DB search  | LSP       | No            |
| `textdoc`                   | Browse project | LSP       | No            |
| `remember_how_to_use_tools` | Save notes     | LSP       | No            |

---

## Development Workflows

### How to Add a New Redux Slice

**1. Create slice file**:

```typescript
// src/features/MyFeature/myFeatureSlice.ts
import { createSlice } from "@reduxjs/toolkit";

export type MyFeatureState = {
  data: string[];
  loading: boolean;
};

const initialState: MyFeatureState = {
  data: [],
  loading: false,
};

export const myFeatureSlice = createSlice({
  name: "myFeature",
  initialState,
  reducers: {
    setData: (state, action: PayloadAction<string[]>) => {
      state.data = action.payload;
    },
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.loading = action.payload;
    },
  },
  selectors: {
    selectData: (state) => state.data,
    selectLoading: (state) => state.loading,
  },
});

export const { setData, setLoading } = myFeatureSlice.actions;
export const { selectData, selectLoading } = myFeatureSlice.selectors;
```

**2. Register in store**:

```typescript
// src/app/store.ts
import { myFeatureSlice } from "../features/MyFeature/myFeatureSlice";

const rootReducer = combineSlices(
  chatSlice,
  historySlice,
  myFeatureSlice, // ← Add here
  // ...
);
```

**3. Use in components**:

```typescript
import { useAppSelector, useAppDispatch } from '@/hooks'
import { selectData, setData } from '@/features/MyFeature/myFeatureSlice'

function MyComponent() {
  const data = useAppSelector(selectData)
  const dispatch = useAppDispatch()

  return (
    <button onClick={() => dispatch(setData(['new']))}>
      Update
    </button>
  )
}
```

### How to Add a New API Endpoint

**Using RTK Query**:

**1. Create API file**:

```typescript
// src/services/refact/myApi.ts
import { createApi } from "@reduxjs/toolkit/query/react";
import { baseQueryWithAuth } from "./index";

export const myApi = createApi({
  reducerPath: "myApi",
  baseQuery: baseQueryWithAuth,
  endpoints: (builder) => ({
    getMyData: builder.query<MyDataResponse, { id: string }>({
      query: ({ id }) => `/v1/my-endpoint/${id}`,
    }),
    updateMyData: builder.mutation<void, { id: string; data: MyData }>({
      query: ({ id, data }) => ({
        url: `/v1/my-endpoint/${id}`,
        method: "POST",
        body: data,
      }),
    }),
  }),
});

export const { useGetMyDataQuery, useUpdateMyDataMutation } = myApi;
```

**2. Register in store**:

```typescript
// src/app/store.ts
import { myApi } from "../services/refact/myApi";

const rootReducer = combineSlices(
  // ... other slices
  myApi, // ← RTK Query auto-registers
);

const store = configureStore({
  reducer: rootReducer,
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware()
      .prepend(listenerMiddleware.middleware)
      .concat(myApi.middleware), // ← Add middleware
});
```

**3. Use in components**:

```typescript
import { useGetMyDataQuery, useUpdateMyDataMutation } from '@/services/refact/myApi'

function MyComponent() {
  const { data, isLoading, error } = useGetMyDataQuery({ id: '123' })
  const [updateData] = useUpdateMyDataMutation()

  return (
    <div>
      {isLoading && <Spinner />}
      {error && <ErrorCallout>{error.message}</ErrorCallout>}
      {data && <div>{data.value}</div>}
    </div>
  )
}
```

### How to Add a New Component

**1. Create component directory**:

```
src/components/MyComponent/
├── MyComponent.tsx
├── MyComponent.module.css
├── MyComponent.stories.tsx
├── MyComponent.test.tsx (optional)
└── index.ts
```

**2. Component file**:

```typescript
// MyComponent.tsx
import React from 'react'
import { Flex, Text } from '@radix-ui/themes'
import styles from './MyComponent.module.css'

export interface MyComponentProps {
  title: string
  onAction?: () => void
}

export function MyComponent({ title, onAction }: MyComponentProps) {
  return (
    <Flex className={styles.container} direction="column" gap="2">
      <Text size="3" weight="medium">{title}</Text>
      {onAction && (
        <button onClick={onAction} className={styles.button}>
          Action
        </button>
      )}
    </Flex>
  )
}
```

**3. CSS Module**:

```css
/* MyComponent.module.css */
.container {
  padding: var(--space-3);
  border-radius: var(--radius-2);
  background: var(--color-surface);
}

.button {
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--gray-6);
  border-radius: var(--radius-2);
  background: var(--accent-3);
  color: var(--accent-11);
  cursor: pointer;
}

.button:hover {
  background: var(--accent-4);
}
```

**4. Storybook story**:

```typescript
// MyComponent.stories.tsx
import type { Meta, StoryObj } from "@storybook/react";
import { MyComponent } from "./MyComponent";

const meta: Meta<typeof MyComponent> = {
  title: "Components/MyComponent",
  component: MyComponent,
  tags: ["autodocs"],
};

export default meta;
type Story = StoryObj<typeof MyComponent>;

export const Default: Story = {
  args: {
    title: "Example Title",
  },
};

export const WithAction: Story = {
  args: {
    title: "Clickable",
    onAction: () => alert("Clicked!"),
  },
};
```

**5. Index file**:

```typescript
// index.ts
export { MyComponent } from "./MyComponent";
export type { MyComponentProps } from "./MyComponent";
```

### How to Add a New Hook

**1. Create hook file**:

```typescript
// src/hooks/useMyHook.ts
import { useState, useEffect } from "react";
import { useAppSelector } from "./useAppSelector";

export function useMyHook(param: string) {
  const [result, setResult] = useState<string | null>(null);
  const config = useAppSelector((state) => state.config);

  useEffect(() => {
    // Hook logic here
    const value = processParam(param, config);
    setResult(value);
  }, [param, config]);

  return result;
}
```

**2. Export from index**:

```typescript
// src/hooks/index.ts
export * from "./useMyHook";
```

**3. Use in components**:

```typescript
import { useMyHook } from '@/hooks'

function MyComponent() {
  const result = useMyHook('input')
  return <div>{result}</div>
}
```

### Project Conventions

**File naming**:

- Components: `PascalCase.tsx`
- Hooks: `useCamelCase.ts`
- Utilities: `camelCase.ts`
- Types: `PascalCase.ts` or `types.ts`
- CSS Modules: `PascalCase.module.css`

**Import order**:

1. React imports
2. Third-party imports
3. Internal imports (features, components, hooks)
4. Types
5. Styles

**TypeScript**:

- Always use types/interfaces (no `any`)
- Prefer `type` over `interface` (unless extending)
- Export types from same file as implementation

**Testing**:

- Test files next to implementation: `MyComponent.test.tsx`
- Use `describe` blocks for grouping
- Mock external dependencies with MSW

---

## Testing

### Testing Stack

- **Framework**: Vitest 3.1
- **React Testing**: React Testing Library 16.0
- **Mocking**: MSW 2.3 (Mock Service Worker)
- **Environment**: happy-dom (lightweight DOM)
- **Coverage**: Vitest coverage-v8

### Test Setup

**Global setup**: `src/utils/test-setup.ts`

```typescript
import { beforeAll, afterEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

beforeAll(() => {
  // Stub browser APIs
  stubResizeObserver();
  stubIntersectionObserver();
  Element.prototype.scrollIntoView = vi.fn();

  // Mock localStorage
  global.localStorage = {
    getItem: vi.fn(() => null),
    setItem: vi.fn(),
    removeItem: vi.fn(),
    clear: vi.fn(),
    key: vi.fn(() => null),
    length: 0,
  };
});

afterEach(() => {
  cleanup(); // Clean up React components
});

// Mock lottie animations
vi.mock("lottie-react", () => ({
  default: vi.fn(),
  useLottie: vi.fn(() => ({
    View: React.createElement("div"),
    playSegments: vi.fn(),
  })),
}));
```

### Custom Render Function

**Location**: `src/utils/test-utils.tsx`

```typescript
import { render as rtlRender } from '@testing-library/react'
import { Provider } from 'react-redux'
import { setUpStore } from '../app/store'

function customRender(
  ui: ReactElement,
  {
    preloadedState,
    store = setUpStore(preloadedState),
    ...renderOptions
  }: ExtendedRenderOptions = {}
) {
  const user = userEvent.setup()

  function Wrapper({ children }: PropsWithChildren) {
    return (
      <Provider store={store}>
        <Theme>
          <TourProvider>
            <AbortControllerProvider>
              {children}
            </AbortControllerProvider>
          </TourProvider>
        </Theme>
      </Provider>
    )
  }

  return {
    ...rtlRender(ui, { wrapper: Wrapper, ...renderOptions }),
    store,
    user
  }
}

export { customRender as render }
export * from '@testing-library/react'
```

**Usage**:

```typescript
import { render, screen, waitFor } from '@/utils/test-utils'

test('renders chat', () => {
  render(<Chat />, {
    preloadedState: {
      chat: { thread: { messages: [] } }
    }
  })
  expect(screen.getByText('Chat')).toBeInTheDocument()
})
```

### MSW Setup

**Worker**: `public/mockServiceWorker.js` (generated by MSW)

**Handlers**: `src/__fixtures__/msw.ts`

```typescript
import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const handlers = [
  http.get("http://127.0.0.1:8001/v1/caps", () => {
    return HttpResponse.json({
      chat_default_model: "gpt-4",
      chat_models: {
        "gpt-4": { n_ctx: 8192 },
      },
    });
  }),

  http.post("http://127.0.0.1:8001/v1/chat", async ({ request }) => {
    const body = await request.json();
    // Return streaming response
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(
          new TextEncoder().encode('data: {"choices":[...]}\n\n'),
        );
        controller.enqueue(new TextEncoder().encode("data: [DONE]\n\n"));
        controller.close();
      },
    });
    return new HttpResponse(stream, {
      headers: { "Content-Type": "text/event-stream" },
    });
  }),
];

export const server = setupServer(...handlers);

// Start server before tests
beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

### Fixtures

**Location**: `src/__fixtures__/`

**20+ fixture files** for test data:

```typescript
// caps.ts
export const STUB_CAPS_RESPONSE = {
  chat_default_model: "gpt-4",
  chat_models: {
    /* ... */
  },
};

// chat.ts
export const STUB_CHAT_MESSAGES = [
  { role: "user", content: "Hello" },
  { role: "assistant", content: "Hi there!" },
];

// tools_response.ts
export const STUB_TOOL_CALL = {
  id: "call_123",
  function: { name: "shell", arguments: '{"cmd":"ls"}' },
};
```

### Example Tests

**Component test**:

```typescript
// ChatForm.test.tsx
import { render, screen, waitFor } from '@/utils/test-utils'
import { ChatForm } from './ChatForm'

describe('ChatForm', () => {
  test('sends message on submit', async () => {
    const { user } = render(<ChatForm />)

    const input = screen.getByRole('textbox')
    await user.type(input, 'Hello AI')

    const button = screen.getByRole('button', { name: /send/i })
    await user.click(button)

    await waitFor(() => {
      expect(screen.getByText('Sending...')).toBeInTheDocument()
    })
  })

  test('disables send when empty', () => {
    render(<ChatForm />)
    const button = screen.getByRole('button', { name: /send/i })
    expect(button).toBeDisabled()
  })
})
```

**Hook test**:

```typescript
// useSendChatRequest.test.ts
import { renderHook, waitFor } from "@testing-library/react";
import { useSendChatRequest } from "./useSendChatRequest";

test("submit sends message", async () => {
  const { result } = renderHook(() => useSendChatRequest());

  act(() => {
    result.current.submit({ question: "Test" });
  });

  await waitFor(() => {
    expect(result.current.isWaiting).toBe(true);
  });
});
```

### Running Tests

```bash
# Watch mode (default)
npm test

# Run once (CI)
npm run test:no-watch

# Coverage report
npm run coverage

# UI mode (visual test runner)
npm run test:ui
```

### Storybook as Dev Tool

**Storybook** serves as visual component documentation:

```bash
npm run storybook  # Start on :6006
```

**30+ stories** across components, showcasing:

- Different states (loading, error, success)
- Edge cases (empty, long text, special chars)
- Interactive controls (change props live)

**Stories use MSW** for API mocking:

```typescript
// ChatContent.stories.tsx
export const Streaming: Story = {
  parameters: {
    msw: {
      handlers: [
        http.post('/v1/chat', () => /* streaming response */)
      ]
    }
  }
}
```

---

## Debugging

### Debug Mode

**Enable logging**:

```bash
DEBUG=refact,app,integrations npm run dev
```

**Debug namespaces**:

- `refact` - Core chat logic
- `app` - Application lifecycle
- `integrations` - Integration system
- `*` - Everything

**Location**: `src/debugConfig.ts`

```typescript
import debug from "debug";

export const debugRefact = debug("refact");
export const debugApp = debug("app");
export const debugIntegrations = debug("integrations");

// Usage in code:
debugRefact("Sending message: %O", message);
```

### Redux DevTools

**Auto-enabled in development**:

```typescript
const store = configureStore({
  reducer: rootReducer,
  middleware: /* ... */,
  devTools: process.env.NODE_ENV !== 'production'  // ← Auto-enabled
})
```

**Features**:

- Time-travel debugging
- Action replay
- State diff viewer
- Performance monitoring

**Max actions**: 50 (configured in store)

### Console Logging Patterns

**Guarded logs** (most of codebase):

```typescript
if (process.env.NODE_ENV === "development") {
  console.log("Debug info:", data);
}
```

**Production logs** (errors only):

```typescript
console.error("Critical error:", error);
```

**~5% of code has console.log** - minimal logging philosophy

### Telemetry

**Location**: `src/services/refact/telemetry.ts`

**What's tracked**:

```typescript
telemetryApi.useSendTelemetryChatEventMutation()

// Events tracked:
{
  scope: 'replaceSelection' | 'ideOpenFile/customization.yaml' | 'copyToClipboard',
  success: boolean,
  error_message: string
}
```

**Telemetry is opt-in** (configured in LSP server)

### Common Issues & Solutions

#### Issue: Messages not sending

**Triage**:

```typescript
// Check these selectors in Redux DevTools:
state.chat.prevent_send; // Should be false
state.chat.waiting_for_response; // Should be false when idle
state.chat.streaming; // Should be false when idle
state.confirmation.pauseReasons; // Should be empty []
```

**Fix**:

- If `prevent_send: true` → Click "Retry" or start new chat
- If paused → Check ToolConfirmation popup, confirm or reject
- If streaming stuck → Reload app

#### Issue: Tool confirmation stuck

**Triage**:

```typescript
state.confirmation.pauseReasons; // What's blocking?
state.confirmation.wasInteracted; // Did user interact?
```

**Fix**:

- Check if IDE sent `ideToolCallResponse`
- Check middleware listener is running
- Confirm/reject manually in UI

#### Issue: Streaming stopped mid-response

**Triage**:

- Check browser console for errors
- Check Network tab for aborted requests
- Check if `doneStreaming` was called prematurely

**Fix**:

- LSP server issue (restart LSP)
- Network interruption (retry)
- Check abort controller logic

#### Issue: Dark mode not working

**Triage**:

```typescript
state.config.themeProps.appearance; // What's set?
document.body.className; // Should be 'vscode-dark' or 'vscode-light'
```

**Fix**:

- Check middleware listener for appearance changes
- Verify Radix Theme is wrapping app
- Check if host is controlling theme

#### Issue: postMessage not working

**Triage**:

```typescript
state.config.host; // Should match actual host
window.acquireVsCodeApi; // Exists in VSCode?
window.postIntellijMessage; // Exists in JetBrains?
```

**Fix**:

- Verify host type is correct
- Check IDE extension is sending messages
- Check event listeners are attached

### Performance Debugging

**React DevTools Profiler**:

- Record chat interaction
- Look for long renders (>16ms)
- Check component re-render count

**Common bottlenecks**:

- Large message arrays (use selectors, not direct state)
- Markdown rendering (memoize with React.memo)
- Recursive renderMessages (optimize with useCallback)

### Network Debugging

**Check requests in Network tab**:

| Endpoint    | Expected Response | Check                       |
| ----------- | ----------------- | --------------------------- |
| `/v1/caps`  | JSON              | 200 OK                      |
| `/v1/chat`  | SSE stream        | 200 OK, `text/event-stream` |
| `/v1/tools` | JSON              | 200 OK                      |

**Common issues**:

- CORS errors → LSP server not running
- 401 Unauthorized → Check `state.config.apiKey`
- Connection refused → Wrong LSP port

### Debug Checklist

When investigating issues:

- [ ] Check Redux state in DevTools
- [ ] Check browser console for errors
- [ ] Check Network tab for failed requests
- [ ] Enable DEBUG logging
- [ ] Check LSP server is running (`:8001/v1/ping`)
- [ ] Verify host type matches environment
- [ ] Check middleware listeners are registered
- [ ] Review recent actions in Redux timeline
- [ ] Check for pause reasons blocking flow
- [ ] Verify messages array structure

---

## Special Features

### Checkpoints System

**Purpose**: Rollback workspace to previous state (undo AI code changes)

**Location**: `src/features/Checkpoints/`

**How it works**:

```
User message → AI makes changes → Checkpoint created
                                        ↓
                               {workspace_folder, commit_hash}
                                        ↓
                            Attached to user message
                                        ↓
                   User clicks 🔄 Reset button
                                        ↓
                     Preview changes (API call)
                                        ↓
                      Apply rollback (API call)
                                        ↓
                 Files reverted + chat truncated
```

**API Endpoints**:

```typescript
// Preview what will change
POST /v1/preview_checkpoints
{
  "checkpoints": [
    { "workspace_folder": "/path", "commit_hash": "abc123" }
  ]
}
// Returns: { files: [{file_name, status: 'A'|'M'|'D'}], error_log: string }

// Apply rollback
POST /v1/restore_checkpoints
{
  "checkpoints": [/* same */]
}
// Returns: { success: boolean, error_log?: string }
```

**UI Components**:

- `CheckpointButton` - Per-message reset button
- `Checkpoints` modal - Shows file changes before apply
- `CheckpointsStatusIndicator` - Visual feedback

**State**:

```typescript
state.checkpoints = {
  previewData: { files: [...], error_log: '' } | null,
  restoreInProgress: boolean
}
```

**After restore**:

- Chat history truncates to checkpoint message
- OR starts new chat with context
- IDE reloads affected files (JetBrains auto-refresh)

### Docker Integration

**Purpose**: Manage Docker containers from chat UI

**Location**: `src/components/IntegrationsView/IntegrationDocker/`

**Features**:

- List containers by image/label
- Start/Stop/Kill/Remove actions
- View environment variables
- SmartLinks for AI context

**API**:

```typescript
// List containers
POST /v1/docker-container-list
{ "docker_image_name": "postgres", "docker_container_labels": ["app=myapp"] }
// Returns: { containers: [{ id, name, status, ports, env, ... }] }

// Execute action
POST /v1/docker-container-action
{ "container_id": "abc123", "action": "start" }
// Returns: { success: boolean, message: string }
```

**UI**:

- `DockerContainerCard` - Shows container details
- Actions dropdown: Start, Stop, Kill, Remove
- Env vars collapsible
- SmartLinks feed container info to AI

**Use case**: AI can reference containers in responses, user manages from UI

### Compression Hints

**Purpose**: Alert user when context is too large

**Indicator**: 🗜️ icon on user messages

**Detection**: LSP returns `compression_strength` in response:

```typescript
type CompressionStrength = "absent" | "weak" | "strong";
```

**When shown**:

- `weak` - Context approaching limit
- `strong` - Context exceeds recommended size

**Action**:

- Show "Start New Chat" suggestion
- User can reject or accept suggestion

**State**:

```typescript
thread.new_chat_suggested = {
  wasSuggested: boolean,
  wasRejectedByUser?: boolean
}
```

### Memory System (Context Files)

**Feature**: AI can remember information across chats

**Indicator**: 🗃️ icon on messages

**How it works**:

1. AI calls `remember_how_to_use_tools()`
2. Notes saved to vector DB
3. Relevant notes attached to future messages
4. Shows as `context_file` messages

**Message type**:

```typescript
type ContextFileMessage = {
  role: "context_file";
  content: ChatContextFile[];
};

type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
};
```

**Rendering**: ContextFiles component shows attached files

### Queued Messages

**Purpose**: Send multiple messages while AI is responding

**How it works**:

- User sends message while streaming → Message queued
- Queue has priority levels:
  - `priority: true` - Send immediately after current stream
  - `priority: false` - Send after tools complete

**State**:

```typescript
type QueuedUserMessage = {
  id: string
  message: UserMessage
  createdAt: number
  priority?: boolean
}

state.chat.queued_messages: QueuedUserMessage[]
```

**Auto-flush** handled by `useAutoSend()` hook

**Visual**: QueuedMessage component shows pending messages

### Multi-Modal Support

**Images in user messages**:

```typescript
{
  role: 'user',
  content: [
    { type: 'text', text: 'What's in this image?' },
    { type: 'image_url', image_url: { url: 'data:image/png;base64,...' } }
  ]
}
```

**Images in tool results**:

```typescript
{
  role: 'tool',
  content: [
    { type: 'image_url', image_url: { url: 'http://...' } }
  ]
}
```

**UI**: `DialogImage` component for lightbox view

### Smart Links

**Purpose**: Context-aware actions in chat

**Format**: Special markdown links

```markdown
[🔗 Open file.py:42](smartlink://open?file=file.py&line=42)
```

**Rendered by**: `SmartLink` component

**Actions**:

- Open file at line
- Run command
- Navigate to integration
- Apply configuration

### Usage Tracking

**Shows in UI**: Token counts, cost estimates

**Data sources**:

```typescript
message.usage = {
  prompt_tokens: number,
  completion_tokens: number,
  total_tokens: number,
  cache_read_input_tokens?: number,
  cache_creation_input_tokens?: number
}

// Metering (coins for SmallCloud)
message.metering_balance?: number
message.metering_*_tokens_n?: number
message.metering_coins_*?: number
```

**Component**: `UsageCounter` - Shows breakdown of token usage

### Reasoning Content

**Feature**: Separate field for model's reasoning (Claude, o1, etc.)

**Format**:

```typescript
{
  role: 'assistant',
  content: 'Here's my answer',           // Main response
  reasoning_content: 'First I thought...' // Reasoning (hidden by default)
}
```

**UI**: `ReasoningContent` component - Collapsible section

### Thinking Blocks

**Feature**: Structured reasoning blocks (different from reasoning_content)

```typescript
type ThinkingBlock = {
  thinking: string; // Reasoning text
  signature?: string; // Model signature/metadata
};

message.thinking_blocks = [{ thinking: "...", signature: "..." }];
```

**Rendered in**: AssistantInput (collapsible)

---

## Quick Reference

### File Structure Cheat Sheet

```
src/
├── app/                 # Redux store, middleware, storage
├── components/          # Reusable UI (40+ components)
├── features/            # Redux slices + feature UIs (25+ features)
├── hooks/               # Custom hooks (60+)
├── services/            # API definitions (refact + smallcloud)
├── events/              # IDE integration types
├── lib/                 # Library entry + render function
├── utils/               # Utility functions
├── __fixtures__/        # Test data (20+ files)
└── debugConfig.ts       # Debug namespaces
```

### Key Commands

```bash
# Development
npm ci                   # Install deps
npm run dev              # Dev server
npm run build            # Build library
npm test                 # Run tests
npm run storybook        # Component explorer
npm run lint             # Lint code
npm run types            # Type check
DEBUG=* npm run dev      # Debug mode

# Publishing
npm run alpha:version    # Bump alpha version
npm run alpha:publish    # Publish to npm
```

### Important Patterns

**Redux**:

- Use selectors (don't access state directly)
- Use RTK Query for APIs
- Use listeners for cross-cutting concerns

**Components**:

- Use Radix primitives + CSS Modules
- Use design tokens (no magic numbers)
- Memoize expensive renders

**Hooks**:

- Export from `hooks/index.ts`
- Use `useAppSelector`/`useAppDispatch` wrappers
- Follow `use` prefix convention

**Types**:

- Use type guards for message routing
- Export types with implementation
- Strict TypeScript mode (no `any`)

### Critical State Invariants

```typescript
// Chat can send if ALL true:
!state.chat.prevent_send
!state.chat.waiting_for_response
!state.chat.streaming
!selectHasUncalledTools(state)
state.confirmation.pauseReasons.length === 0

// Tool confirmation needed if:
lastMessage.tool_calls exists
!wasInteracted
!(isPatchLike && automatic_patch)

// Queue flushes when:
// Priority: base conditions (no streaming, no waiting)
// Regular: base + no tools + no pause reasons
```

### Common Gotchas

1. **Don't mutate state** - Redux Toolkit allows in reducers, but not elsewhere
2. **Don't skip selectors** - Always use memoized selectors
3. **Don't bypass type guards** - Use `isAssistantMessage()` etc.
4. **Don't hardcode colors/spacing** - Use Radix tokens
5. **Don't forget to register** - New slices/APIs must be registered in store
6. **Don't block the UI** - Use abort controllers for cancellable requests
7. **Don't trust streaming order** - Handle out-of-order chunks
8. **Don't forget pause reasons** - Tool confirmation can block everything

### Debugging Quick Wins

```typescript
// Check state in console:
window.__REDUX_DEVTOOLS_EXTENSION__;

// Force re-render:
dispatch(newChatAction());

// Clear pause:
dispatch(
  clearPauseReasonsAndHandleToolsStatus({
    wasInteracted: false,
    confirmationStatus: true,
  }),
);

// Reset prevent_send:
dispatch(enableSend({ id: chatId }));

// Check LSP health:
fetch("http://127.0.0.1:8001/v1/ping").then((r) => r.json());
```

---

## For AI Coding Agents

### When Modifying Message Flow

**MUST CHECK**:

1. State transitions (`waiting_for_response`, `streaming`, `prevent_send`)
2. Tool confirmation logic (don't break pause system)
3. Queue flush conditions (priority vs regular)
4. Abort handling (cleanup state properly)
5. Message formatting (use `formatChatResponse`)
6. Type guards (don't assume message structure)

### When Adding Message Types

**MUST DO**:

1. Add type definition in `services/refact/types.ts`
2. Add type guard (`isMyMessage`)
3. Update `formatChatResponse` to handle it
4. Update `renderMessages` to render it
5. Create component for rendering
6. Update `formatMessagesForLsp` if needed for sending

### When Touching Redux

**MUST DO**:

1. Use selectors (create if missing)
2. Use immutable updates (even though Immer allows mutations)
3. Add to `combineSlices` if new slice
4. Add middleware if new RTK Query API
5. Test state transitions

### When Modifying UI

**MUST DO**:

1. Use Radix primitives where possible
2. Use CSS Modules (not inline styles)
3. Use design tokens (not literals)
4. Test dark mode
5. Check responsive (at least 768px)
6. Add Storybook story

### Red Flags

🚨 **STOP if you see**:

- Direct state mutation outside reducers
- Hardcoded colors (#hex) or spacing (px)
- `any` types (use proper typing)
- Synchronous network calls (use async)
- Missing type guards for message routing
- Global CSS without `:global()` wrapper
- Missing cleanup in `useEffect` returns

---

## Version History

**Current**: v2.0.10-alpha.3

**Recent changes** (inferred from codebase):

- Queued messages with priority system
- Compression hints and new chat suggestions
- Reasoning content support
- Tool confirmation improvements
- Docker integration enhancements
- Checkpoints UI polish

---

## Contributing

### Before Submitting PR

- [ ] Run `npm run lint` (no errors)
- [ ] Run `npm run types` (type check passes)
- [ ] Run `npm test` (all tests pass)
- [ ] Add tests for new features
- [ ] Add Storybook story for new components
- [ ] Update AGENTS.md if architecture changes
- [ ] Follow existing code style
- [ ] No console.log in production code

### Commit Messages

Follow conventional commits:

```
feat: add queued messages
fix: prevent double-send on tool confirmation
refactor: extract streaming logic
docs: update AGENTS.md
test: add tool loop prevention test
```

---

## Getting Help

**Resources**:

- README.md - Library API reference
- Storybook - Component documentation (`:6006`)
- Redux DevTools - State inspection
- GitHub Issues - Bug reports

**Community**:

- GitHub: https://github.com/smallcloudai/refact
- Discord: (check README)

---

**Last Updated**: December 2024  
**Document Version**: 1.0  
**Maintained by**: SmallCloudAI Team

---

_This document is a living guide. If you find errors or omissions, please update it._
