const REFACT_URL = "http://127.0.0.1:8001";
const CHAT_URL = `${REFACT_URL}/v1/chat`;

export type ChatRole = "user" | "assistant" | "context_file";
export type ChatMessage = [ChatRole, string];
export type ChatMessages = ChatMessage[];

interface BaseDelta {
  role: ChatRole;
}

interface UserDelta extends BaseDelta {
  role: "user";
  content: string;
}

interface AssistantDelta extends BaseDelta {
  role: "assistant";
  content: string;
}

interface ChatContextFile extends BaseDelta {
  role: "context_file";
  file_content: string;
}

type Delta = UserDelta | AssistantDelta | ChatContextFile;
// interface Delta extends UserDelta, AssistantDelta , ChatContextFile {}

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

const API_KEY: string | undefined = import.meta.env.VITE_REFACT_API_KEY;
if (!API_KEY) {
  // eslint-disable-next-line no-console
  console.error("VITE_REFACT_API_KEY not configured in .env file");
  throw new Error("api-key not defined");
}

export function sendChat(
  messages: ChatMessages,
  model: string,
  abortController: AbortController,
) {
  const jsonMessages = messages.map(([role, content]) => {
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
    Authorization: `Bearer ${API_KEY}`,
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
