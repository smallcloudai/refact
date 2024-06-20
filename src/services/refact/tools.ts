import { getApiKey } from "../../utils/ApiKey";
import { AT_TOOLS_AVAILABLE_URL } from "./consts";

export type ToolParams = {
  name: string;
  type: string;
  description: string;
};

export type ToolFunction = {
  name: string;
  description: string;
  parameters: ToolParams[];
  parameters_required: string[];
};

export type ToolCommand = {
  function: ToolFunction;
  type: "function";
};

export async function getAvailableTools(
  lspUrl?: string,
): Promise<ToolCommand[]> {
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
    return [];
  }

  // TODO: add type guards
  return (await response.json()) as unknown as ToolCommand[];
}
