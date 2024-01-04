const REFACT_URL = import.meta.env.VITE_REFACT_LSP_URL ?? "";
const CHAT_URL = `${REFACT_URL}/v1/chat`;
const CAPS_URL = `${REFACT_URL}/v1/caps`;

export type ChatRole = "user" | "assistant" | "context_file";
export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
};

export type ChatContentFileMessage = ["context_file", ChatContextFile];
export type ChatMessage =
  | [Omit<ChatRole, "context_file">, string]
  | ChatContentFileMessage;

export type ChatMessages = ChatMessage[];

export function isChatContextFileMessage(
  message: ChatMessage,
): message is ChatContentFileMessage {
  return message[0] === "context_file";
}

interface BaseDelta {
  role: ChatRole;
}

interface AssistantDelta extends BaseDelta {
  role: "assistant";
  content: string;
}

// TODO: confirm UserDelta and ContextFileDelta are sent frm the lsp
interface ChatContextFileDelta extends BaseDelta {
  role: "context_file";
  file_content: string;
}

interface UserDelta extends BaseDelta {
  role: "user";
  content: string;
}

type Delta = UserDelta | AssistantDelta | ChatContextFileDelta;
// interface Delta extends UserDelta, AssistantDelta , ChatContextFile {}

export type ChatChoice = {
  delta: Delta; // TODO: so far I've only seen AssistantDelta come from the lsp
  finish_reason: "stop" | "abort" | null;
  index: number;
};

export type ChatResponse = {
  choices: ChatChoice[];
  created: number;
  model: string;
  id: string;
};

export function sendChat(
  messages: ChatMessages,
  model: string,
  abortController: AbortController,
) {
  const jsonMessages = messages.map(([role, textOrFile]) => {
    const content =
      typeof textOrFile === "string"
        ? textOrFile
        : JSON.stringify([textOrFile]);
    return { role, content };
  });

  const body = JSON.stringify({
    messages: jsonMessages,
    model: model,
    parameters: {
      max_new_tokens: 1000,
    },
    stream: true,
  });

  const headers = {
    "Content-Type": "application/json",
  };

  return fetch(CHAT_URL, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    signal: abortController.signal,
  });
}

export async function getCaps(): Promise<CapsResponse> {
  const response = await fetch(CAPS_URL, {
    method: "GET",
    headers: {
      accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();

  if (!isCapsResponse(json)) {
    throw new Error("Invalid response from caps");
  }

  return json;
}

type CodeChatModel = {
  default_scratchpad: string;
  n_ctx: number;
  similar_models: string[];
  supports_scratchpads: Record<
    string,
    {
      default_system_message: string;
    }
  >;
};

type CodeCompletionModel = {
  default_scratchpad: string;
  n_ctx: number;
  similar_models: string[];
  supports_scratchpads: Record<string, Record<string, unknown>>;
};

export function isCapsResponse(json: unknown): json is CapsResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("code_chat_default_model" in json)) return false;
  if (typeof json.code_chat_default_model !== "string") return false;
  if (!("code_chat_models" in json)) return false;
  return true;
}

export type CapsResponse = {
  caps_version: number;
  cloud_name: string;
  code_chat_default_model: string;
  code_chat_models: Record<string, CodeChatModel>;
  code_completion_default_model: string;
  code_completion_models: Record<string, CodeCompletionModel>;
  code_completion_n_ctx: number;
  endpoint_chat_passthrough: string;
  endpoint_style: string;
  endpoint_template: string;
  running_models: string[];
  telemetry_basic_dest: string;
  telemetry_corrected_snippets_dest: string;
  tokenizer_path_template: string;
  tokenizer_rewrite_path: Record<string, unknown>;
};
