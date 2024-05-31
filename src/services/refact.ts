import { getApiKey } from "../utils/ApiKey";
const CHAT_URL = `/v1/chat`;
const CAPS_URL = `/v1/caps`;
const STATISTIC_URL = `/v1/get-dashboard-plots`;
const AT_COMMAND_COMPLETION = "/v1/at-command-completion";
const AT_COMMAND_PREVIEW = "/v1/at-command-preview";
const CUSTOM_PROMPTS_URL = "/v1/customization";

const AT_TOOLS_AVAILABLE_URL = "/v1/at-tools-available";

export type ChatRole =
  | "user"
  | "assistant"
  | "context_file"
  | "system"
  | "tool";

export type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
  usefulness?: number;
  // FIXME: typo in lsp
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
};

export type ToolResult = {
  tool_call_id: string;
  finish_reason: string; // "call_failed" | "call_worked";
  content: string;
};

interface BaseMessage
  extends Array<
    string | ChatContextFile[] | ToolCall[] | ToolResult | undefined | null
  > {
  0: ChatRole;
  1: null | string | ChatContextFile[] | ToolResult;
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
  2?: ToolCall[];
}

export interface SystemMessage extends BaseMessage {
  0: "system";
  1: string;
}

export interface ToolMessage extends BaseMessage {
  0: "tool";
  1: ToolResult;
}

export function isUserMessage(message: ChatMessage): message is UserMessage {
  return message[0] === "user";
}

export type ChatMessage =
  | UserMessage
  | AssistantMessage
  | ChatContextFileMessage
  | SystemMessage
  | ToolMessage;

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

interface BaseDelta {
  role?: ChatRole | null;
}

interface AssistantDelta extends BaseDelta {
  role?: "assistant" | null;
  content: string | null; // might be undefined, will be null if tool_calls
}

export function isAssistantDelta(delta: unknown): delta is AssistantDelta {
  if (!delta) return false;
  if (typeof delta !== "object") return false;
  // if (!("role" in delta)) return false;
  if ("role" in delta && delta.role === "assistant") return true;
  if (!("content" in delta)) return false;
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
}

export function isToolCallDelta(delta: unknown): delta is ToolCallDelta {
  if (!delta) return false;
  if (typeof delta !== "object") return false;
  if (!("tool_calls" in delta)) return false;
  return Array.isArray(delta.tool_calls);
}

type Delta = AssistantDelta | ChatContextFileDelta | ToolCallDelta;

export type ChatChoice = {
  delta: Delta;
  finish_reason: "stop" | "abort" | "tool_calls" | null;
  index: number;
};

export type ChatUserMessageResponse = {
  id: string;
  role: "user" | "context_file";
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
  return json.role === "user" || json.role === "context_file";
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

const _TOOLS = [
  {
    function: {
      description:
        "Find definition of a symbol in a project using AST. Symbol could be: function, method, class, type alias.",
      name: "definition",
      parameters: {
        properties: {
          symbol: {
            description:
              "The name of the symbol (function, method, class, type alias) to find within the project.",
            type: "string",
          },
        },
        required: ["symbol"],
        type: "object",
      },
    },
    type: "function",
  },
  {
    function: {
      description:
        "Read the file located using given file_path and provide its content",
      name: "file",
      parameters: {
        properties: {
          file_path: {
            description:
              "absolute path to the file or filename to be found within the project.",
            type: "string",
          },
        },
        required: ["file_path"],
        type: "object",
      },
    },
    type: "function",
  },
  {
    function: {
      description: "Compile the project",
      name: "compile",
      parameters: {
        properties: {},
        required: [],
        type: "object",
      },
    },
    type: "function",
  },
];

export async function sendChat(
  messages: ChatMessages,
  model: string,
  abortController: AbortController,
  stream: boolean | undefined = true,
  lspUrl?: string,
) {
  const jsonMessages = messages.reduce<
    {
      role: string;
      content: string | null;
      tool_calls?: Omit<ToolCall, "index">[];
      tool_call_id?: string;
    }[]
  >((acc, message) => {
    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message[0],
          content: message[1],
          tool_calls: message[2],
        },
      ]);
    }

    if (isToolMessage(message)) {
      return acc.concat([
        {
          role: "tool",
          content: message[1].content,
          tool_call_id: message[1].tool_call_id,
        },
      ]);
    }

    const content =
      typeof message[1] === "string" ? message[1] : JSON.stringify(message[1]);
    return [...acc, { role: message[0], content }];
  }, []);

  const toolsResponse = await getAvailableTools();

  const body = JSON.stringify({
    messages: jsonMessages,
    model: model,
    parameters: {
      max_new_tokens: 2048,
    },
    stream,
    // stream: false,
    tools: toolsResponse,
    // tools: TOOLS, // works
    // tools: [], // causes bugs
    // tools: toolsResponse.slice(0, 1), // can cause bugs
    // tools: toolsResponse.slice(1) // can work
    max_tokens: 2048,
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

export type CommandCompletionResponse = {
  completions: string[];
  replace: [number, number];
  is_cmd_executable: boolean;
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

export type ContextBucket = {
  file_path: string;
  line1: number;
  line2: number;
  name: string;
};

export type Buckets = ContextBucket[];

export type FIMContext = {
  attached_files?: ContextFiles;

  bucket_declarations?: Buckets;
  bucket_usage_of_same_stuff?: Buckets;
  bucket_high_overlap?: Buckets;
  cursor_symbols?: Buckets;

  fim_ms?: number;
  n_ctx?: number;
  rag_ms?: number;
  rag_tokens_limit?: number;
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

type AtParamDict = {
  name: string;
  type: string;
  description: string;
};

type AtToolFunction = {
  name: string;
  description: string;
  parameters: AtParamDict[];
  parameters_required: string[];
};

type AtToolCommand = {
  function: AtToolFunction;
  type: "function";
};

type AtToolResponse = AtToolCommand[];

export async function getAvailableTools(
  lspUrl?: string,
): Promise<AtToolResponse> {
  const toolsUrl = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${AT_TOOLS_AVAILABLE_URL}`
    : AT_TOOLS_AVAILABLE_URL;

  const apiKey = getApiKey();

  const response = await fetch(toolsUrl, {
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

  // TODO: add type guards
  return (await response.json()) as unknown as AtToolResponse;
}
