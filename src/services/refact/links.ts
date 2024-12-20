import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";
import { ChatMessages } from "./types";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";
import { CHAT_COMMIT_LINK_URL, CHAT_LINKS_URL } from "./consts";
import { LspChatMode } from "../../features/Chat";
// useful for forcing specific links
// import { STUB_LINKS_FOR_CHAT_RESPONSE } from "../../__fixtures__";

// goto: can be an integration file to open in settings, a file to open in an idea or a global integration.
// XXX rename to:
// link_text
// link_goto
// link_action
// link_tooltip
type LinkActions =
  | "patch-all"
  | "follow-up"
  | "commit"
  | "goto"
  | "summarize-project";

export type ChatLink = BaseLink | CommitLink;

interface BaseLink {
  link_action: LinkActions;
  link_text: string;
  link_goto?: string;
  link_tooltip?: string;
  link_payload?: CommitLinkPayload | null;
  link_summary_path?: string;
}
export interface CommitLink extends BaseLink {
  link_text: string;
  link_action: "commit";
  link_goto: string;
  link_tooltip: string;
  link_payload: CommitLinkPayload;
}

export type CommitLinkPayload = {
  project_path: string;
  commit_message: string;
  file_changes: { path: string; status: string }[];
};

function isChatLink(json: unknown): json is ChatLink {
  if (!json || typeof json !== "object") return false;

  if (!("link_action" in json) || typeof json.link_action !== "string") {
    return false;
  }

  if (!("link_text" in json)) return false;
  if (typeof json.link_text !== "string") return false;

  if ("link_goto" in json && typeof json.link_goto !== "string") return false;

  return true;
}

export function isCommitLink(chatLink: ChatLink): chatLink is CommitLink {
  return "link_action" in chatLink && chatLink.link_action === "commit";
}

export type LinksForChatResponse = {
  links: ChatLink[];
  uncommited_changes_warning: string;
};

export type LinksApiRequest = {
  chat_id: string;
  messages: ChatMessages;
  model: string;
  mode?: LspChatMode;
  current_config_file?: string;
};

function isLinksForChatResponse(json: unknown): json is LinksForChatResponse {
  if (!json || typeof json !== "object") return false;
  if (!("links" in json)) return false;
  if (!Array.isArray(json.links)) return false;
  return json.links.every(isChatLink);
}

export const linksApi = createApi({
  reducerPath: "linksApi",
  tagTypes: ["Chat_Links"],
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, { getState }) => {
      const token = (getState() as RootState).config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getLinksForChat: builder.query<LinksForChatResponse, LinksApiRequest>({
      providesTags: ["Chat_Links"],
      async queryFn(args, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const messageFotLsp = formatMessagesForLsp(args.messages);

        const response = await baseQuery({
          ...extraOptions,
          method: "POST",
          url: `http://127.0.0.1:${port}${CHAT_LINKS_URL}`,
          body: {
            meta: {
              chat_id: args.chat_id,
              current: args.current_config_file,
              chat_mode: args.mode,
            },
            messages: messageFotLsp,
            model_name: args.model,
          },
        });

        if (response.error) {
          return { error: response.error };
        }

        if (!isLinksForChatResponse(response.data)) {
          return {
            error: {
              error: "Invalid response for chat links",
              data: response.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        // return { data: STUB_LINKS_FOR_CHAT_RESPONSE };

        return { data: response.data };
      },
    }),

    sendCommit: builder.mutation<CommitResponse, CommitLinkPayload>({
      invalidatesTags: ["Chat_Links"],
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${CHAT_COMMIT_LINK_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          method: "POST",
          url,
          body: arg,
        });
        if (response.error) {
          return { error: response.error };
        }

        if (!isCommitResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              data: response.data,
              error: "Invalid response for commit",
            },
          };
        }

        return { data: response.data };
      },
    }),
  }),
});

export type CommitResponse = {
  commits_applied: {
    project_path: string;
    project_name: string;
    commit_oid: string;
  }[];
  error_log: {
    error_message: string;
    project_path: string;
    project_name: string;
  }[];
};

function isCommitResponse(json: unknown): json is CommitResponse {
  if (!json || typeof json !== "object") return false;
  if (!("commits_applied" in json)) return false;
  if (!Array.isArray(json.commits_applied)) return false;
  if (!("error_log" in json)) return false;
  if (!Array.isArray(json.error_log)) return false;
  // TODO: type check the arrays if we use the data anywhere.
  return true;
}
