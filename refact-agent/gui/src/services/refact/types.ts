import { LspChatMode } from "../../features/Chat";
import { Checkpoint } from "../../features/Checkpoints/types";
import { GetChatTitleActionPayload, GetChatTitleResponse, Usage } from "./chat";
import { MCPArgs, MCPEnvs } from "./integrations";

export type ChatRole =
  | "user"
  | "assistant"
  | "context_file"
  | "system"
  | "tool"
  | "diff"
  | "plain_text"
  | "cd_instruction";

export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
  cursor?: number;
  usefulness?: number;
  usefullness?: number;
};

export type ToolCall = {
  function: {
    arguments: string; // stringed json
    name?: string; // will be present when it's new
  };
  index: number;
  type?: "function";
  id?: string;
  attached_files?: string[];
  subchat?: string;
};

export type ToolUsage = {
  functionName: string;
  amountOfCalls: number;
};

function isToolCall(call: unknown): call is ToolCall {
  if (!call) return false;
  if (typeof call !== "object") return false;
  if (!("function" in call)) return false;
  if (!("index" in call)) return false;
  return true;
}

export const validateToolCall = (toolCall: ToolCall) => {
  if (!isToolCall(toolCall)) return false;
  try {
    JSON.parse(toolCall.function.arguments);
    return true;
  } catch {
    return false;
  }
};

type ToolContent = string | MultiModalToolContent[];

export function isToolContent(json: unknown): json is ToolContent {
  if (!json) return false;
  if (typeof json === "string") return true;
  if (Array.isArray(json)) return json.every(isMultiModalToolContent);
  return false;
}
export interface BaseToolResult {
  tool_call_id: string;
  finish_reason?: string; // "call_failed" | "call_worked";
  content: ToolContent;
}

export interface SingleModelToolResult extends BaseToolResult {
  content: string;
}
export interface MultiModalToolResult extends BaseToolResult {
  content: MultiModalToolContent[];
}

export type ToolResult = SingleModelToolResult | MultiModalToolResult;

export type MultiModalToolContent = {
  m_type: string; // "image/*" | "text" ... maybe narrow this?
  m_content: string; // base64 if image,
};

export function isMultiModalToolContent(
  content: unknown,
): content is MultiModalToolContent {
  if (!content) return false;
  if (typeof content !== "object") return false;
  if (!("m_type" in content)) return false;
  if (typeof content.m_type !== "string") return false;
  if (!("m_content" in content)) return false;
  if (typeof content.m_content !== "string") return false;
  return true;
}

export function isMultiModalToolContentArray(content: ToolContent) {
  if (!Array.isArray(content)) return false;
  return content.every(isMultiModalToolContent);
}

export function isMultiModalToolResult(
  toolResult: ToolResult,
): toolResult is MultiModalToolResult {
  return isMultiModalToolContentArray(toolResult.content);
}

export function isSingleModelToolResult(toolResult: ToolResult) {
  return typeof toolResult.content === "string";
}

interface BaseMessage {
  role: ChatRole;
  content:
    | string
    | ChatContextFile[]
    | ToolResult
    | DiffChunk[]
    | null
    | (UserMessageContentWithImage | ProcessedUserMessageContentWithImages)[];
}

function isBaseMessage(json: unknown): json is BaseMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  if (typeof json.role !== "string") return false;
  if (!("content" in json)) return false;
  return true;
}

export interface ChatContextFileMessage extends BaseMessage {
  role: "context_file";
  content: ChatContextFile[];
}

export type UserImage = {
  type: "image_url";
  image_url: { url: string };
};

export type UserMessageContentWithImage =
  | {
      type: "text";
      text: string;
    }
  | UserImage;
export interface UserMessage extends BaseMessage {
  role: "user";
  content:
    | string
    | (UserMessageContentWithImage | ProcessedUserMessageContentWithImages)[];
  checkpoints?: Checkpoint[];
}

export type ProcessedUserMessageContentWithImages = {
  m_type: string;
  m_content: string;
};
export interface AssistantMessage extends BaseMessage {
  role: "assistant";
  content: string | null;
  tool_calls?: ToolCall[] | null;
  finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
}

export interface ToolCallMessage extends AssistantMessage {
  tool_calls: ToolCall[];
}

export interface SystemMessage extends BaseMessage {
  role: "system";
  content: string;
}

export interface ToolMessage extends BaseMessage {
  role: "tool";
  content: ToolResult;
}

// TODO: There maybe sub-types for this
export type DiffChunk = {
  file_name: string;
  file_action: string;
  line1: number;
  line2: number;
  lines_remove: string;
  lines_add: string;
  file_name_rename?: string | null;
  application_details?: string;
  // apply?: boolean;
  // chunk_id?: number;
};

export function isDiffChunk(json: unknown) {
  if (!json) {
    return false;
  }
  if (typeof json !== "object") {
    return false;
  }
  if (!("file_name" in json) || typeof json.file_name !== "string") {
    return false;
  }
  if (!("file_action" in json) || typeof json.file_action !== "string") {
    return false;
  }
  if (!("line1" in json) || typeof json.line1 !== "number") {
    return false;
  }
  if (!("line2" in json) || typeof json.line2 !== "number") {
    return false;
  }
  if (!("lines_remove" in json) || typeof json.lines_remove !== "string") {
    return false;
  }
  if (!("lines_add" in json) || typeof json.lines_add !== "string") {
    return false;
  }
  return true;
}
export interface DiffMessage extends BaseMessage {
  role: "diff";
  content: DiffChunk[];
  tool_call_id: string;
}

export function isUserMessage(message: BaseMessage): message is UserMessage {
  return message.role === "user";
}

export interface PlainTextMessage extends BaseMessage {
  role: "plain_text";
  content: string;
}

export interface CDInstructionMessage extends BaseMessage {
  role: "cd_instruction";
  content: string;
}

export type ChatMessage =
  | UserMessage
  | AssistantMessage
  | ChatContextFileMessage
  | SystemMessage
  | ToolMessage
  | DiffMessage
  | PlainTextMessage
  | CDInstructionMessage;

export type ChatMessages = ChatMessage[];

export type ChatMeta = {
  current_config_file?: string | undefined;
  chat_id?: string | undefined;
  request_attempt_id?: string | undefined;
  chat_mode: LspChatMode;
};

export function isChatContextFileMessage(
  message: ChatMessage,
): message is ChatContextFileMessage {
  return message.role === "context_file";
}

export function isAssistantMessage(
  message: ChatMessage,
): message is AssistantMessage {
  return message.role === "assistant";
}

export function isToolMessage(message: ChatMessage): message is ToolMessage {
  return message.role === "tool";
}

export function isDiffMessage(message: ChatMessage): message is DiffMessage {
  return message.role === "diff";
}

export function isSystemMessage(
  message: ChatMessage,
): message is SystemMessage {
  return message.role === "system";
}

export function isToolCallMessage(
  message: ChatMessage,
): message is ToolCallMessage {
  if (!isAssistantMessage(message)) return false;
  const tool_calls = message.tool_calls;
  if (!tool_calls) return false;
  // TODO: check browser support of every
  return tool_calls.every(isToolCall);
}

export function isPlainTextMessage(
  message: ChatMessage,
): message is PlainTextMessage {
  return message.role === "plain_text";
}

export function isCDInstructionMessage(
  message: ChatMessage,
): message is CDInstructionMessage {
  return message.role === "cd_instruction";
}

export function isAChatMessage(value: unknown) {
  if (!value) return false;
  if (typeof value !== "object") return false;
}

// function isChatMessage(json: unknown): json is ChatMessage {
//   if (!json) return false;
//   if (typeof json !== "object") return false;
//   if (!("role" in json)) return false;
//   if(typeof json.role !== "string") return false;
//   if(isChatContextFileMessage(json)) return false;

// }

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
  finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
  index: number;
};

export type ChatUserMessageResponse =
  | {
      id: string;
      role: "user" | "context_file" | "context_memory";
      content: string;
      checkpoints?: Checkpoint[];
    }
  | {
      id: string;
      role: "user";
      content:
        | string
        | (
            | UserMessageContentWithImage
            | ProcessedUserMessageContentWithImages
          )[];
      checkpoints?: Checkpoint[];
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

export type UserMessageResponse = ChatUserMessageResponse & {
  role: "user";
};

export function isChatGetTitleResponse(
  json: unknown,
): json is GetChatTitleResponse {
  if (!json || typeof json !== "object") return false;

  const requiredKeys = [
    "id",
    "choices",
    // "metering_balance", // not in BYOK
    "model",
    "object",
    "system_fingerprint",
    "usage",
    "created",
    "deterministic_messages",
  ];

  return requiredKeys.every((key) => key in json);
}

export function isChatGetTitleActionPayload(
  json: unknown,
): json is GetChatTitleActionPayload {
  if (!json || typeof json !== "object") return false;

  const requiredKeys = ["title", "chatId"];

  return requiredKeys.every((key) => key in json);
}

export function isUserResponse(json: unknown): json is UserMessageResponse {
  if (!isChatUserMessageResponse(json)) return false;
  return json.role === "user";
}

export type ContextFileResponse = ChatUserMessageResponse & {
  role: "context_file";
};

export function isContextFileResponse(
  json: unknown,
): json is ContextFileResponse {
  if (!isChatUserMessageResponse(json)) return false;
  return json.role === "context_file";
}

export type SubchatContextFileResponse = {
  content: string;
  role: "context_file";
};

export function isSubchatContextFileResponse(
  json: unknown,
): json is SubchatContextFileResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("content" in json)) return false;
  if (!("role" in json)) return false;
  return json.role === "context_file";
}

export type ContextMemoryResponse = ChatUserMessageResponse & {
  role: "context_memory";
};

export function isContextMemoryResponse(
  json: unknown,
): json is ContextMemoryResponse {
  if (!isChatUserMessageResponse(json)) return false;
  return json.role === "context_memory";
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
  tool_call_id: string;
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
  tool_call_id: string;
  tool_calls?: ToolCall[];
}

export function isPlainTextResponse(json: unknown): json is PlainTextResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  return json.role === "plain_text";
}

export type SubchatResponse = {
  add_message: ChatResponse;
  subchat_id: string;
  tool_call_id: string;
};

export function isSubchatResponse(json: unknown): json is SubchatResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("add_message" in json)) return false;
  if (!("subchat_id" in json)) return false;
  if (!("tool_call_id" in json)) return false;
  return true;
}

export function isSystemResponse(json: unknown): json is SystemMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  return json.role === "system";
}

export function isCDInstructionResponse(
  json: unknown,
): json is CDInstructionMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  return json.role === "cd_instruction";
}

type ChatResponseChoice = {
  choices: ChatChoice[];
  created: number;
  model: string;
  id: string;
  usage?: Usage;
  refact_agent_request_available?: null | number;
  refact_agent_max_request_num?: number;
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
  | ToolResponse
  | PlainTextResponse;

export function areAllFieldsBoolean(
  json: unknown,
): json is Record<string, boolean> {
  return (
    typeof json === "object" &&
    json !== null &&
    Object.values(json).every((value) => typeof value === "boolean")
  );
}

export type MemoRecord = {
  memid: string;
  thevec?: number[]; // are options nullable?
  distance?: number;
  m_type: string;
  m_goal: string;
  m_project: string;
  m_payload: string;
  m_origin: string;
  // mstat_correct: bigint,
  // mstat_relevant: bigint,
  mstat_correct: number;
  mstat_relevant: number;
  mstat_times_used: number;
};
export function isMemoRecord(obj: unknown): obj is MemoRecord {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("memid" in obj) || typeof obj.memid !== "string") return false;
  // TODO: other checks
  return true;
}

export type VecDbStatus = {
  files_unprocessed: number;
  files_total: number; // only valid for status bar in the UI, resets to 0 when done
  requests_made_since_start: number;
  vectors_made_since_start: number;
  db_size: number;
  db_cache_size: number;
  state: "starting" | "parsing" | "done" | "cooldown";
  queue_additions: boolean;
  vecdb_max_files_hit: boolean;
  vecdb_errors: Record<string, number>;
};

export function isVecDbStatus(obj: unknown): obj is VecDbStatus {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (
    !("files_unprocessed" in obj) ||
    typeof obj.files_unprocessed !== "number"
  ) {
    return false;
  }
  if (!("files_total" in obj) || typeof obj.files_total !== "number") {
    return false;
  }
  if (
    !("requests_made_since_start" in obj) ||
    typeof obj.requests_made_since_start !== "number"
  ) {
    return false;
  }
  if (
    !("vectors_made_since_start" in obj) ||
    typeof obj.vectors_made_since_start !== "number"
  ) {
    return false;
  }
  if (!("db_size" in obj) || typeof obj.db_size !== "number") {
    return false;
  }
  if (!("db_cache_size" in obj) || typeof obj.db_cache_size !== "number") {
    return false;
  }

  if (!("state" in obj) || typeof obj.state !== "string") {
    return false;
  }
  if (!("queue_additions" in obj) || typeof obj.queue_additions !== "boolean") {
    return false;
  }
  if (
    !("vecdb_max_files_hit" in obj) ||
    typeof obj.vecdb_max_files_hit !== "boolean"
  ) {
    return false;
  }
  if (!("vecdb_errors" in obj) || typeof obj.vecdb_errors !== "object") {
    return false;
  }

  return true;
}
export function isMCPArgumentsArray(json: unknown): json is MCPArgs {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!Array.isArray(json)) return false;
  if (!json.every((arg) => typeof arg === "string")) return false;
  return true;
}

export function isMCPEnvironmentsDict(json: unknown): json is MCPEnvs {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (Array.isArray(json)) return false;

  return Object.values(json).every((value) => typeof value === "string");
}

// ChatDB

export type CThread = {
  cthread_id: string;
  cthread_belongs_to_chore_event_id: string | null;
  cthread_title: string;
  cthread_toolset: string;
  cthread_model: string;
  cthread_temperature: number;
  cthread_n_ctx: number;
  cthread_max_new_tokens: number;
  cthread_n: number;
  cthread_error: string;
  cthread_anything_new: boolean;
  cthread_created_ts: number;
  cthread_updated_ts: number;
  cthread_archived_ts: number;
  cthread_locked_by: string;
  cthread_locked_ts: number;
};

// export type CThreadDefault = Partial<CThread> & {
//   cthread_id: null | string;
//   cthread_title: string;
//   cthread_toolset: string;
//   cthread_model: string;
// };
export function isCThread(value: unknown): value is CThread {
  if (!value || typeof value !== "object") {
    return false;
  }

  if (!("cthread_id" in value) || typeof value.cthread_id !== "string") {
    return false;
  }

  if (
    !("cthread_belongs_to_chore_event_id" in value) ||
    (value.cthread_belongs_to_chore_event_id !== null &&
      typeof value.cthread_belongs_to_chore_event_id !== "string")
  ) {
    return false;
  }

  if (!("cthread_title" in value) || typeof value.cthread_title !== "string") {
    return false;
  }

  if (
    !("cthread_toolset" in value) ||
    typeof value.cthread_toolset !== "string"
  ) {
    return false;
  }

  if (!("cthread_model" in value) || typeof value.cthread_model !== "string") {
    return false;
  }

  if (
    !("cthread_temperature" in value) ||
    typeof value.cthread_temperature !== "number"
  ) {
    return false;
  }

  if (!("cthread_n_ctx" in value) || typeof value.cthread_n_ctx !== "number") {
    return false;
  }

  if (
    !("cthread_max_new_tokens" in value) ||
    typeof value.cthread_max_new_tokens !== "number"
  ) {
    return false;
  }

  if (!("cthread_n" in value) || typeof value.cthread_n !== "number") {
    return false;
  }

  if (!("cthread_error" in value) || typeof value.cthread_error !== "string") {
    return false;
  }

  if (
    !("cthread_anything_new" in value) ||
    typeof value.cthread_anything_new !== "boolean"
  ) {
    return false;
  }

  if (
    !("cthread_created_ts" in value) ||
    typeof value.cthread_created_ts !== "number"
  ) {
    return false;
  }

  if (
    !("cthread_updated_ts" in value) ||
    typeof value.cthread_updated_ts !== "number"
  ) {
    return false;
  }

  if (
    !("cthread_archived_ts" in value) ||
    typeof value.cthread_archived_ts !== "number"
  ) {
    return false;
  }

  if (
    !("cthread_locked_by" in value) ||
    typeof value.cthread_locked_by !== "string"
  ) {
    return false;
  }

  if (
    !("cthread_locked_ts" in value) ||
    typeof value.cthread_locked_ts !== "number"
  ) {
    return false;
  }

  return true;
}

type CThreadSubResponseUpdate = {
  sub_event: "cthread_update";
  cthread_rec: CThread;
};

export function isCThreadSubResponseUpdate(
  value: unknown,
): value is CThreadSubResponseUpdate {
  if (!value || typeof value !== "object") return false;
  if (!("sub_event" in value)) return false;
  if (typeof value.sub_event !== "string") return false;
  if (!("cthread_rec" in value)) return false;
  return isCThread(value.cthread_rec);
}

type CThreadSubResponseDelete = {
  sub_event: "cthread_delete";
  cthread_id: string;
};

export function isCThreadSubResponseDelete(
  value: unknown,
): value is CThreadSubResponseDelete {
  if (!value || typeof value !== "object") return false;
  if (!("sub_event" in value)) return false;
  if (typeof value.sub_event !== "string") return false;
  if (!("cthread_id" in value)) return false;
  if (typeof value.cthread_id !== "string") return false;
  return true;
}

export type CMessageFromChatDB = {
  cmessage_belongs_to_cthread_id: string;
  cmessage_alt: number;
  cmessage_num: number;
  cmessage_prev_alt: number;
  cmessage_usage_model: string;
  cmessage_usage_prompt: number;
  cmessage_usage_completion: number;
  cmessage_json: string; // stringified json ChatMessage
};

export function isCMessageFromChatDB(
  value: unknown,
): value is CMessageFromChatDB {
  if (!value || typeof value !== "object") return false;
  if (!("cmessage_belongs_to_cthread_id" in value)) return false;
  if (typeof value.cmessage_belongs_to_cthread_id !== "string") return false;
  if (!("cmessage_alt" in value)) return false;
  if (typeof value.cmessage_alt !== "number") return false;
  if (!("cmessage_num" in value)) return false;
  if (typeof value.cmessage_num !== "number") return false;
  if (!("cmessage_prev_alt" in value)) return false;
  if (typeof value.cmessage_prev_alt !== "number") return false;
  if (!("cmessage_usage_model" in value)) return false;
  if (typeof value.cmessage_usage_model !== "string") return false;
  if (!("cmessage_usage_prompt" in value)) return false;
  if (typeof value.cmessage_usage_prompt !== "number") return false;
  if (!("cmessage_usage_completion" in value)) return false;
  if (typeof value.cmessage_usage_completion !== "number") return false;
  if (!("cmessage_json" in value)) return false;
  if (typeof value.cmessage_json !== "string") return false;
  return true;
}

export type CMessage = Omit<CMessageFromChatDB, "cmessage_json"> & {
  cmessage_json: ChatMessage;
};

export type UserCMessage = Omit<CMessage, "cmessage_json"> & {
  cmessage_json: UserMessage;
};

export function isUserCMessage(value: unknown): value is UserCMessage {
  if (!value || typeof value !== "object") return false;
  if (!("cmessage_belongs_to_cthread_id" in value)) return false;
  if (typeof value.cmessage_belongs_to_cthread_id !== "string") return false;
  if (!("cmessage_alt" in value)) return false;
  if (typeof value.cmessage_alt !== "number") return false;
  if (!("cmessage_num" in value)) return false;
  if (typeof value.cmessage_num !== "number") return false;
  if (!("cmessage_prev_alt" in value)) return false;
  if (typeof value.cmessage_prev_alt !== "number") return false;
  if (!("cmessage_usage_model" in value)) return false;
  if (typeof value.cmessage_usage_model !== "string") return false;
  if (!("cmessage_usage_prompt" in value)) return false;
  if (typeof value.cmessage_usage_prompt !== "number") return false;
  if (!("cmessage_usage_completion" in value)) return false;
  if (typeof value.cmessage_usage_completion !== "number") return false;
  if (!("cmessage_json" in value)) return false;
  if (!value.cmessage_json) return false;
  if (!isBaseMessage(value.cmessage_json)) return false;
  if (!isUserMessage(value.cmessage_json)) return false;
  return true;
}

export type CMessageUpdateResponse = {
  sub_event: "cmessage_update";
  cmessage_rec: CMessageFromChatDB;
};

export function isCMessageUpdateResponse(
  value: unknown,
): value is CMessageUpdateResponse {
  if (!value || typeof value !== "object") return false;
  if (!("sub_event" in value)) return false;
  if (typeof value.sub_event !== "string") return false;
  if (value.sub_event !== "cmessage_update") return false;
  if (!("cmessage_rec" in value)) return false;
  return isCMessageFromChatDB(value.cmessage_rec);
}
