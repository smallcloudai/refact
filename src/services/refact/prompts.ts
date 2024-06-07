import { getApiKey } from "../../utils/ApiKey";
import { CUSTOM_PROMPTS_URL } from "./consts";

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
