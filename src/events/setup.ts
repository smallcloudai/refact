export enum EVENT_NAMES_FROM_SETUP {
  SETUP_HOST = "setup_host",
  OPEN_EXTERNAL_URL = "open_external_url",
}

export interface CloudHost {
  type: "cloud";
  apiKey: string;
  sendCorrectedCodeSnippets: boolean;
}

export interface SelfHost {
  type: "self";
  endpointAddress: string;
}

export interface EnterpriseHost {
  type: "enterprise";
  endpointAddress: string;
  apiKey: string;
}

export interface ActionFromSetup {
  type: EVENT_NAMES_FROM_SETUP;
  payload?: Record<string, unknown>;
}

export type HostSettings = CloudHost | SelfHost | EnterpriseHost;

export function isActionFromSetup(action: unknown): action is ActionFromSetup {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_FROM_SETUP,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export interface SetupHost extends ActionFromSetup {
  type: EVENT_NAMES_FROM_SETUP.SETUP_HOST;
  payload: { host: HostSettings };
}

export function isSetupHost(action: unknown): action is SetupHost {
  if (!isActionFromSetup(action)) return false;
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  return action.type === EVENT_NAMES_FROM_SETUP.SETUP_HOST;
}

export interface OpenExternalUrl extends ActionFromSetup {
  type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL;
  payload: { url: string };
}

export function isOpenExternalUrl(action: unknown): action is OpenExternalUrl {
  if (!isActionFromSetup(action)) return false;
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  return action.type === EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL;
}
