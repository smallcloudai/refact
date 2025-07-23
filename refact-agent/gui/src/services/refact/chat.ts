import {
  ChatRole,
  ThinkingBlock,
  ToolCall,
  ToolMessage,
  UserMessage,
} from "./types";

export const DEFAULT_MAX_NEW_TOKENS = 4096;

export type LSPUserMessage = Pick<
  UserMessage,
  "checkpoints" | "compression_strength"
> & {
  role: UserMessage["ftm_role"];
  content: UserMessage["ftm_content"];
};

export type LSPToolMessage = {
  role: "tool";
  content: ToolMessage["ftm_content"];
  tool_call_id: string;
};

export type LspChatMessage =
  | {
      role: ChatRole;
      // TODO make this a union type for user message
      content: string | null;
      finish_reason?: "stop" | "length" | "abort" | "tool_calls" | null;
      // TBD: why was index omitted ?
      // tool_calls?: Omit<ToolCall, "index">[];
      thinking_blocks?: ThinkingBlock[];
      tool_calls?: ToolCall[];
      tool_call_id?: string;
      usage?: Usage | null;
    }
  | LSPUserMessage
  | LSPToolMessage
  | { role: string; content: string };

// could be more narrow.
export function isLspChatMessage(json: unknown): json is LspChatMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("role" in json)) return false;
  if (typeof json.role !== "string") return false;
  if (!("content" in json)) return false;
  if (json.content !== null && typeof json.content !== "string") return false;
  return true;
}

export function isLspUserMessage(
  message: LspChatMessage,
): message is LSPUserMessage {
  return message.role === "user";
}

export type GetChatTitleResponse = {
  choices: Choice[];
  created: number;
  deterministic_messages: DeterministicMessage[];
  id: string;
  metering_balance: number;
  model: string;
  object: string;
  system_fingerprint: string;
  usage: Usage;
};

export type GetChatTitleActionPayload = {
  chatId: string;
  title: string;
};

export type Choice = {
  finish_reason: string;
  index: number;
  message: Message;
};

export type Message = {
  content: string;
  role: string;
};

export type DeterministicMessage = {
  content: string;
  role: string;
  tool_call_id: string;
  usage: unknown;
};

export type CompletionTokenDetails = {
  accepted_prediction_tokens: number | null;
  audio_tokens: number | null;
  reasoning_tokens: number | null;
  rejected_prediction_tokens: number | null;
};

export type PromptTokenDetails = {
  audio_tokens: number | null;
  cached_tokens: number;
};

// TODO: check this
export type Usage = {
  // completion_tokens: number;
  // prompt_tokens: number;
  // total_tokens: number;
  // completion_tokens_details?: CompletionTokenDetails | null;
  // prompt_tokens_details?: PromptTokenDetails | null;
  // cache_creation_input_tokens?: number;
  // cache_read_input_tokens?: number;
  coins: number;
  tokens_prompt: number;
  pp1000t_prompt: number;
  tokens_cache_read: number;
  tokens_completion: number;
  pp1000t_cache_read: number;
  pp1000t_completion: number;
  tokens_prompt_text: number;
  tokens_prompt_audio: number;
  tokens_prompt_image: number;
  tokens_prompt_cached: number;
  tokens_cache_creation: number;
  pp1000t_cache_creation: number;
  tokens_completion_text: number;
  tokens_completion_audio: number;
  tokens_completion_reasoning: number;
  pp1000t_completion_reasoning: number;
};

export function isUsage(usage: unknown): usage is Usage {
  if (!usage || typeof usage !== "object") return false;

  // if (!("completion_tokens" in usage)) return false;
  // if (typeof usage.completion_tokens !== "number") return false;
  // if (!("prompt_tokens" in usage)) return false;
  // if (typeof usage.prompt_tokens !== "number") return false;
  // if (!("total_tokens" in usage)) return false;
  // if (typeof usage.total_tokens !== "number") return false;

  const requiredFields: (keyof Usage)[] = [
    "coins",
    "tokens_prompt",
    "pp1000t_prompt",
    "tokens_cache_read",
    "tokens_completion",
    "pp1000t_cache_read",
    "pp1000t_completion",
    "tokens_prompt_text",
    "tokens_prompt_audio",
    "tokens_prompt_image",
    "tokens_prompt_cached",
    "tokens_cache_creation",
    "pp1000t_cache_creation",
    "tokens_completion_text",
    "tokens_completion_audio",
    "tokens_completion_reasoning",
    "pp1000t_completion_reasoning",
  ];

  for (const field of requiredFields) {
    if (!(field in usage)) return false;
    if (typeof (usage as Usage)[field] !== "number") return false;
  }

  return true;
}
