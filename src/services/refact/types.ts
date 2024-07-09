export type ChatRole =
  | "user"
  | "assistant"
  | "context_file"
  | "system"
  | "tool"
  | "context_memory"
  | "diff"
  | "plain_text";

export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
  usefulness?: number;
  usefullness?: number;
};

export type ContextMemory = {
  memo_id: string;
  memo_text: string;
};

export type ToolCall = {
  function: {
    arguments: string; // stringed json
    name?: string; // will be present when it's new
  };
  index: number;
  type?: "function";
  id?: string;
};

function isToolCall(call: unknown): call is ToolCall {
  if (!call) return false;
  if (typeof call !== "object") return false;
  if (!("function" in call)) return false;
  if (!("index" in call)) return false;
  return true;
}

export type ToolResult = {
  tool_call_id: string;
  finish_reason?: string; // "call_failed" | "call_worked";
  content: string;
};

interface BaseMessage
  extends Array<
    | string
    | ChatContextFile[]
    | ToolCall[]
    | ToolResult
    | undefined
    | null
    | ContextMemory[]
    | DiffChunk[]
  > {
  0: ChatRole;
  1:
    | null
    | string
    | ChatContextFile[]
    | ToolResult
    | ContextMemory[]
    | DiffChunk[];
}

export interface ChatContextFileMessage extends BaseMessage {
  0: "context_file";
  1: ChatContextFile[];
}

export interface UserMessage extends BaseMessage {
  0: "user";
  1: string;
}

export interface AssistantMessage extends BaseMessage {
  0: "assistant";
  1: string | null;
  2?: ToolCall[] | null;
}

export interface ToolCallMessage extends AssistantMessage {
  2: ToolCall[];
}

export interface SystemMessage extends BaseMessage {
  0: "system";
  1: string;
}

export interface ToolMessage extends BaseMessage {
  0: "tool";
  1: ToolResult;
}

export interface MemoryMessage extends BaseMessage {
  0: "context_memory";
  1: ContextMemory[];
}

// TODO: There maybe sub-types for this
export type DiffChunk = {
  file_name: string;
  file_action: string;
  line1: number;
  line2: number;
  lines_remove: string;
  lines_add: string;
  apply?: boolean;
  chunk_id?: number;
};
export interface DiffMessage extends BaseMessage {
  0: "diff";
  1: DiffChunk[];
}

export function isUserMessage(message: ChatMessage): message is UserMessage {
  return message[0] === "user";
}

export interface PlainTextMessage extends BaseMessage {
  0: "plain_text";
  1: string;
}
/**
 * {"content":"[{\"memo_id\":\"about_frog_class.txt\",\"memo_text\":\"The Frog class is used to test the project, located in tests folder.\\n\"},{\"memo_id\":\"answer_in_spanish.txt\",\"memo_text\":\"User actually preferes answers in Spanish, so do that.\\n\"}]","role":"context_memory","tool_call_id":"","tool_calls":null}
 */

export type ChatMessage =
  | UserMessage
  | AssistantMessage
  | ChatContextFileMessage
  | SystemMessage
  | ToolMessage
  | MemoryMessage
  | DiffMessage
  | PlainTextMessage;

export type ChatMessages = ChatMessage[];

export function isChatContextFileMessage(
  message: ChatMessage,
): message is ChatContextFileMessage {
  return message[0] === "context_file";
}

export function isAssistantMessage(
  message: ChatMessage,
): message is AssistantMessage {
  return message[0] === "assistant";
}

export function isToolMessage(message: ChatMessage): message is ToolMessage {
  return message[0] === "tool";
}

export function isDiffMessage(message: ChatMessage): message is DiffMessage {
  return message[0] === "diff";
}

export function isToolCallMessage(
  message: ChatMessage,
): message is ToolCallMessage {
  if (!isAssistantMessage(message)) return false;
  const tool_calls = message[2];
  if (!tool_calls) return false;
  // TODO: check browser support of evey
  return tool_calls.every(isToolCall);
}

export function isPlainTextMessage(
  message: ChatMessage,
): message is PlainTextMessage {
  return message[0] === "plain_text";
}

interface BaseDelta {
  role?: ChatRole | null;
}

interface AssistantDelta extends BaseDelta {
  role?: "assistant" | null;
  content?: string | null; // might be undefined, will be null if tool_calls
  tool_calls?: ToolCall[];
}

export function isAssistantDelta(delta: unknown): delta is AssistantDelta {
  if (!delta) return false;
  if (typeof delta !== "object") return false;
  if ("role" in delta && delta.role !== null && delta.role !== "assistant")
    return false;
  if (!("content" in delta)) return false;
  if (typeof delta.content !== "string") return false;
  return true;
}
interface ChatContextFileDelta extends BaseDelta {
  role: "context_file";
  content: ChatContextFile[];
}

export function isChatContextFileDelta(
  delta: unknown,
): delta is ChatContextFileDelta {
  if (!delta) return false;
  if (typeof delta !== "object") return false;
  if (!("role" in delta)) return false;
  return delta.role === "context_file";
}

interface ToolCallDelta extends BaseDelta {
  tool_calls: ToolCall[];
  content?: string | null;
}

export function isToolCallDelta(delta: unknown): delta is ToolCallDelta {
  if (!delta) return false;
  if (typeof delta !== "object") return false;
  if (!("tool_calls" in delta)) return false;
  if (delta.tool_calls === null) return false;
  return Array.isArray(delta.tool_calls);
}

type Delta = AssistantDelta | ChatContextFileDelta | ToolCallDelta | BaseDelta;

export type ChatChoice = {
  delta: Delta;
  finish_reason?: "stop" | "abort" | "tool_calls" | null;
  index: number;
};

export type ChatUserMessageResponse = {
  id: string;
  role: "user" | "context_file" | "context_memory";
  content: string;
};

export type ToolResponse = {
  id: string;
  role: "tool";
} & ToolResult;

export function isChatUserMessageResponse(
  json: unknown,
): json is ChatUserMessageResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("id" in json)) return false;
  if (!("content" in json)) return false;
  if (!("role" in json)) return false;
  return (
    json.role === "user" ||
    json.role === "context_file" ||
    json.role === "context_memory"
  );
}

export function isToolResponse(json: unknown): json is ToolResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  // if (!("id" in json)) return false;
  if (!("content" in json)) return false;
  if (!("role" in json)) return false;
  if (!("tool_call_id" in json)) return false;
  return json.role === "tool";
}

export type DiffResponse = {
  role: "diff";
  content: string;
};

export function isDiffResponse(json: unknown): json is DiffResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("content" in json)) return false;
  if (!("role" in json)) return false;
  return json.role === "diff";
}
export interface PlainTextResponse {
  role: "plain_text";
  content: string;
}

export function isPlainTextResponse(json: unknown): json is PlainTextResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  return json.role === "plain_text";
}

type ChatResponseChoice = {
  choices: ChatChoice[];
  created: number;
  model: string;
  id: string;
};

export function isChatResponseChoice(
  res: ChatResponse,
): res is ChatResponseChoice {
  if (!("choices" in res)) return false;
  return true;
}

export type ChatResponse =
  | ChatResponseChoice
  | ChatUserMessageResponse
  | ToolResponse;
