import { getApiKey } from "../utils/ApiKey";
const CHAT_URL = `/v1/chat`;
const CAPS_URL = `/v1/caps`;
const STATISTIC_URL = `/v1/get-dashboard-plots`;
const AT_COMMAND_COMPLETION = "/v1/at-command-completion";
const AT_COMMAND_PREVIEW = "/v1/at-command-preview";
const CUSTOM_PROMPTS_URL = "/v1/customization";

export type ChatRole = "user" | "assistant" | "context_file" | "system";

export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
  usefulness?: number;
  // FIXME: typo in lsp
  usefullness?: number;
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
  | ChatContextFileMessage
  | SystemMessage;

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

export type ChatUserMessageResponse = {
  id: string;
  role: "user" | "context_file";
  content: string;
};

export function isChatUserMessageResponse(
  json: unknown,
): json is ChatUserMessageResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("id" in json)) return false;
  if (!("content" in json)) return false;
  if (!("role" in json)) return false;
  return json.role === "user" || json.role === "context_file";
}

export type ChatResponse =
  | {
      choices: ChatChoice[];
      created: number;
      model: string;
      id: string;
    }
  | ChatUserMessageResponse;

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

  const apiKey = getApiKey();
  const headers = {
    "Content-Type": "application/json",
    ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
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

export function isStatisticDataResponse(
  json: unknown,
): json is { data: string } {
  if (!json || typeof json !== "object") return false;
  if (!("data" in json)) return false;
  return typeof json.data === "string";
}

export async function getStatisticData(
  lspUrl?: string,
): Promise<{ data: string }> {
  const statisticDataEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${STATISTIC_URL}`
    : STATISTIC_URL;
  const response = await fetch(statisticDataEndpoint, {
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
  if (!isStatisticDataResponse(json)) {
    throw new Error("Invalid response for statistic data");
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

interface Replace {
  0: number;
  1: number;
}

export type CommandCompletionResponse = {
  completions: string[];
  replace: Replace;
  is_cmd_executable: false;
};

export function isCommandCompletionResponse(
  json: unknown,
): json is CommandCompletionResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("completions" in json)) return false;
  if (!("replace" in json)) return false;
  if (!("is_cmd_executable" in json)) return false;
  return true;
}
export type DetailMessage = {
  detail: string;
};
export function isDetailMessage(json: unknown): json is DetailMessage {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("detail" in json)) return false;
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
  if (!isCommandCompletionResponse(json) && !isDetailMessage(json)) {
    throw new Error("Invalid response from completion");
  }

  if (isDetailMessage(json)) {
    return {
      completions: [],
      replace: [0, 0],
      is_cmd_executable: false,
    };
  }

  return json;
}

export type CommandPreviewContent = {
  content: string;
  role: "context_file";
};
export type CommandPreviewResponse = {
  messages: CommandPreviewContent[];
};

export function isCommandPreviewResponse(
  json: unknown,
): json is CommandPreviewResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("messages" in json)) return false;
  if (!Array.isArray(json.messages)) return false;

  if (!json.messages.length) return true;

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

  if (!isCommandPreviewResponse(json) && !isDetailMessage(json)) {
    throw new Error("Invalid response from command preview");
  }

  if (isDetailMessage(json)) {
    return [];
  }

  const jsonMessages = json.messages.map<ChatContextFileMessage>(
    ({ role, content }) => {
      const fileData = JSON.parse(content) as ChatContextFile[];
      return [role, fileData];
    },
  );

  return jsonMessages;
}

export type RefactTableImpactDateObj = {
  completions: number;
  human: number;
  langs: string[];
  refact: number;
  refact_impact: number;
  total: number;
};
export type RefactTableImpactLanguagesRow = {
  [key in ColumnName]: string | number;
};
export type StatisticData = {
  refact_impact_dates: {
    data: {
      daily: Record<string, RefactTableImpactDateObj>;
      weekly: Record<string, RefactTableImpactDateObj>;
    };
  };
  table_refact_impact: {
    columns: string[];
    data: RefactTableImpactLanguagesRow[];
    title: string;
  };
};

export type ColumnName =
  | "lang"
  | "refact"
  | "human"
  | "total"
  | "refact_impact"
  | "completions";

export type CellValue = string | number;

export type FormatCellValue = (
  columnName: string,
  cellValue: string | number,
) => string | number;

export type SystemPrompt = {
  text: string;
  description: string;
};

function isSystemPrompt(json: unknown): json is SystemPrompt {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("text" in json)) return false;
  if (!("description" in json)) return false;
  return true;
}

export type SystemPrompts = Record<string, SystemPrompt>;

export function isSystemPrompts(json: unknown): json is SystemPrompts {
  if (!json) return false;
  if (typeof json !== "object") return false;
  for (const value of Object.values(json)) {
    if (!isSystemPrompt(value)) return false;
  }
  return true;
}

export type CustomPromptsResponse = {
  system_prompts: SystemPrompts;
  toolbox_commands: Record<string, unknown>;
};

export function isCustomPromptsResponse(
  json: unknown,
): json is CustomPromptsResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("system_prompts" in json)) return false;
  if (typeof json.system_prompts !== "object") return false;
  if (json.system_prompts === null) return false;
  return isSystemPrompts(json.system_prompts);
}

export async function getPrompts(lspUrl?: string): Promise<SystemPrompts> {
  const customPromptsUrl = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${CUSTOM_PROMPTS_URL}`
    : CUSTOM_PROMPTS_URL;

  const apiKey = getApiKey();

  const response = await fetch(customPromptsUrl, {
    method: "GET",
    credentials: "same-origin",
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    headers: {
      accept: "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();

  if (!isCustomPromptsResponse(json)) {
    return {};
  }

  return json.system_prompts;
}

type FimChoices = {
  code_completion: string;
  finish_reason: string;
  index: number;
}[];

type FimFile = {
  file_content: string;
  file_name: string;
  line1: number;
  line2: number;
};

type ContextFiles = FimFile[];

export type ContextQueries = {
  from: "declarations" | "cursor_symbols" | "usages";
  symbol: string;
}[];

export type FIMContext = {
  attached_files?: ContextFiles;
  was_looking_for?: ContextQueries;
};

export type FimDebugData = {
  choices: FimChoices;
  snippet_telemetry_id: number;
  model: string;
  context?: FIMContext;
  created?: number;
  elapsed?: number;
  cached?: boolean;
};

// {
//     "choices": [
//         {
//             "code_completion": "export type PromptSelectProps = {\n  value: string;\n  onChange: (value: string) => void;\n  options: string[];\n  disabled?: boolean;\n};",
//             "finish_reason": "stop",
//             "index": 0
//         }
//     ],
//     "context": [],
//     "created": 1712248098.165,
//     "model": "starcoder2/7b/vllm",
//     "snippet_telemetry_id": 109
// }
