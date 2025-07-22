import { FThreadMessageSubs } from "../../../../generated/documents";

export type IntegrationMeta = {
  name?: string;
  path?: string;
  project?: string;
  shouldIntermediatePageShowUp?: boolean;
};

export function isIntegrationMeta(json: unknown): json is IntegrationMeta {
  if (!json || typeof json !== "object") return false;
  if (!("name" in json) || !("path" in json) || !("project" in json)) {
    return false;
  }
  return true;
}

export interface MessageWithIntegrationMeta
  extends Omit<
    FThreadMessageSubs["news_payload_thread_message"],
    "ftm_user_preferences"
  > {
  ftm_user_preferences: { integration: IntegrationMeta };
}

export function isMessageWithIntegrationMeta(
  message: unknown,
): message is MessageWithIntegrationMeta {
  if (!message || typeof message !== "object") return false;
  if (!("ftm_user_preferences" in message)) return false;
  if (
    !message.ftm_user_preferences ||
    typeof message.ftm_user_preferences !== "object"
  )
    return false;
  const preferences = message.ftm_user_preferences as Record<string, unknown>;
  if (!("integration" in preferences)) return false;
  return isIntegrationMeta(preferences.integration);
}

export type LspChatMode =
  | "NO_TOOLS"
  | "EXPLORE"
  | "AGENT"
  | "CONFIGURE"
  | "PROJECT_SUMMARY";
