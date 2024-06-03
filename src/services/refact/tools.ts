import { getApiKey } from "../../utils/ApiKey";
import { AT_TOOLS_AVAILABLE_URL } from "./consts";

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

export type AtToolResponse = AtToolCommand[];

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
