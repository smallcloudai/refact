const CHAT_URL = `/v1/chat`;
const CAPS_URL = `/v1/caps`;
const AT_COMMAND_COMPLETION = "/v1/at-command-completion";
const AT_COMMAND_PREVIEW = "/v1/at-command-preview";

export type ChatRole = "user" | "assistant" | "context_file" | "system";

export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
};

interface BaseMessage extends Array<string | ChatContextFile[]> {
  0: ChatRole;
  1: string | ChatContextFile[];
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
  1: string;
}

export interface SystemMessage extends BaseMessage {
  0: "system";
  1: string;
}

export function isUserMessage(message: ChatMessage): message is UserMessage {
  return message[0] === "user";
}

export type ChatMessage =
  | UserMessage
  | AssistantMessage
  | ChatContextFileMessage;

export type ChatMessages = ChatMessage[];

export function isChatContextFileMessage(
  message: ChatMessage,
): message is ChatContextFileMessage {
  return message[0] === "context_file";
}

interface BaseDelta {
  role: ChatRole;
}

interface AssistantDelta extends BaseDelta {
  role: "assistant";
  content: string;
}
interface ChatContextFileDelta extends BaseDelta {
  role: "context_file";
  content: ChatContextFile[];
}

// interface UserDelta extends BaseDelta {
//   role: "user";
//   content: string;
// }

type Delta = AssistantDelta | ChatContextFileDelta;

export type ChatChoice = {
  delta: Delta;
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
  lspUrl?: string,
) {
  const jsonMessages = messages.map(([role, textOrFile]) => {
    const content =
      typeof textOrFile === "string" ? textOrFile : JSON.stringify(textOrFile);
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
  const chatEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${CHAT_URL}`
    : CHAT_URL;

  return fetch(chatEndpoint, {
    method: "POST",
    headers,
    body,
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    signal: abortController.signal,
    credentials: "same-origin",
  });
}

export async function getCaps(lspUrl?: string): Promise<CapsResponse> {
  const capsEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${CAPS_URL}`
    : CAPS_URL;

  const response = await fetch(capsEndpoint, {
    method: "GET",
    credentials: "same-origin",
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
  chat_rag_functions?: string[];
};

interface Replace {
  0: number;
  1: number;
}

export type CommandCompletionResponse = {
  completions: string[];
  replace: Replace;
  is_cmd_executable: false;
};

function isCommandCompletionResponse(
  json: unknown,
): json is CommandCompletionResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("completions" in json)) return false;
  if (!("replace" in json)) return false;
  if (!("is_cmd_executable" in json)) return false;
  return true;
}

export async function getAtCommandCompletion(
  query: string,
  cursor: number,
  number: number,
  lspUrl?: string,
): Promise<CommandCompletionResponse> {
  const completionEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${AT_COMMAND_COMPLETION}`
    : AT_COMMAND_COMPLETION;

  const response = await fetch(completionEndpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query, cursor, top_n: number }),
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();
  if (!isCommandCompletionResponse(json)) {
    throw new Error("Invalid response from completion");
  }

  return json;
}

type CommandPreviewContent = {
  content: string;
  role: "context_file";
};
export type ResponseFromCommandPreview = {
  messages: CommandPreviewContent[];
};

function isCommandPreviewResponse(
  json: unknown,
): json is ResponseFromCommandPreview {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("messages" in json)) return false;
  if (!Array.isArray(json.messages)) return false;
  if (!json.messages.length) return false;

  const firstMessage: unknown = json.messages[0];
  if (!firstMessage) return false;
  if (typeof firstMessage !== "object") return false;
  if (!("role" in firstMessage)) return false;
  if (firstMessage.role !== "context_file") return false;
  if (!("content" in firstMessage)) return false;
  if (typeof firstMessage.content !== "string") return false;

  return true;
}

export async function getAtCommandPreview(
  query: string,
  lspUrl?: string,
): Promise<ChatContextFileMessage[]> {
  // check this
  const previewEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${AT_COMMAND_PREVIEW}`
    : AT_COMMAND_PREVIEW;

  const response = await fetch(previewEndpoint, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    credentials: "same-origin",
    body: JSON.stringify({ query }),
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();

  if (!isCommandPreviewResponse(json)) {
    throw new Error("Invalid response from command preview");
  }

  const jsonMessages = json.messages.map<ChatContextFileMessage>(
    ({ role, content }) => {
      const fileData = JSON.parse(content) as ChatContextFile[];
      return [role, fileData];
    },
  );

  return jsonMessages;
}
