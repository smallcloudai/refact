import { IntegrationMeta, LspChatMode } from "../../features/Chat";
import { CHAT_URL } from "./consts";
// import { ToolCommand } from "./tools";
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
  | {
      role: "tool";
      content: ToolMessage["ftm_content"];
      tool_call_id: string;
    };

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

type StreamArgs =
  | {
      stream: true;
      abortSignal: AbortSignal;
    }
  | { stream: false; abortSignal?: undefined | AbortSignal };

type SendChatArgs = {
  messages: LspChatMessage[];
  last_user_message_id?: string; // used for `refact-message-id` header
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  port?: number;
  apiKey?: string | null;
  // isConfig?: boolean;
  toolsConfirmed?: boolean;
  checkpointsEnabled?: boolean;
  integration?: IntegrationMeta | null;
  mode?: LspChatMode; // used for chat actions
  boost_reasoning?: boolean;
  increase_max_tokens?: boolean;
} & StreamArgs;

type GetChatTitleArgs = {
  messages: LspChatMessage[];
  model: string;
  lspUrl?: string;
  takeNote?: boolean;
  onlyDeterministicMessages?: boolean;
  chatId?: string;
  port?: number;
  apiKey?: string | null;
  boost_reasoning?: boolean;
} & StreamArgs;

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

// TODO: add config url
export async function sendChat({
  messages,
  model,
  abortSignal,
  stream,
  // lspUrl,
  // takeNote = false,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  port = 8001,
  apiKey,
  checkpointsEnabled = true,
  // isConfig = false,
  integration,
  last_user_message_id = "",
  mode,
  boost_reasoning,
  increase_max_tokens = false,
}: SendChatArgs): Promise<Response> {
  // const toolsResponse = await getAvailableTools();

  // const tools = takeNote
  //   ? toolsResponse.filter(
  //       (tool) => tool.function.name === "remember_how_to_use_tools",
  //     )
  //   : toolsResponse.filter(
  //       (tool) => tool.function.name !== "remember_how_to_use_tools",
  //     );

  const body = JSON.stringify({
    messages,
    model: model,
    stream,
    only_deterministic_messages,
    checkpoints_enabled: checkpointsEnabled,
    // chat_id,
    parameters: boost_reasoning ? { boost_reasoning: true } : undefined,
    increase_max_tokens: increase_max_tokens,
    meta: {
      chat_id,
      request_attempt_id: last_user_message_id,
      // chat_remote,
      // TODO: pass this through
      chat_mode: mode ?? "EXPLORE",
      // chat_mode: "EXPLORE", // NOTOOLS, EXPLORE, AGENT, CONFIGURE, PROJECTSUMMARY,
      // TODO: not clear, that if we set integration.path it's going to be set also in meta as current_config_file
      ...(integration?.path ? { current_config_file: integration.path } : {}),
    },
  });

  //   const apiKey = getApiKey();
  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  };

  const url = `http://127.0.0.1:${port}${CHAT_URL}`;

  return fetch(url, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    // TODO: causes an error during tests :/
    // referrer: "no-referrer",
    signal: abortSignal,
    credentials: "same-origin",
  });
}

export async function generateChatTitle({
  messages,
  stream,
  model,
  onlyDeterministicMessages: only_deterministic_messages,
  chatId: chat_id,
  port = 8001,
  apiKey,
}: GetChatTitleArgs): Promise<Response> {
  const body = JSON.stringify({
    messages,
    model,
    stream,
    max_tokens: 300,
    only_deterministic_messages: only_deterministic_messages,
    chat_id,
    // NOTE: we don't want to use reasoning here, for example Anthropic requires at least max_tokens=1024 for thinking
    // parameters: boost_reasoning ? { boost_reasoning: true } : undefined,
  });

  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
  };

  const url = `http://127.0.0.1:${port}${CHAT_URL}`;

  return fetch(url, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    // TODO: causes an error during tests :/
    // referrer: "no-referrer",
    credentials: "same-origin",
  });
}
