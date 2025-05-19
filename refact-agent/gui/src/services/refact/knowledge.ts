import { RootState } from "../../app/store";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";
import {
  COMPRESS_MESSAGES_URL,
  KNOWLEDGE_CREATE_URL,
  SET_ACTIVE_WORKSPACE_ID,
} from "./consts";
import { isDetailMessage, type ChatMessages } from ".";
import { type Workspace } from "../smallcloud/types";

export type SubscribeArgs =
  | {
      quick_search?: string;
      limit?: number;
    }
  | undefined;

export type MemAddRequest = {
  goal: string;
  payload: string;
  mem_type?: string;
  project?: string;
  origin?: string;
};

export function isAddMemoryRequest(obj: unknown): obj is MemAddRequest {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  // if (!("mem_type" in obj) || typeof obj.mem_type !== "string") return false;
  if (!("goal" in obj) || typeof obj.goal !== "string") return false;
  // if (!("project" in obj) || typeof obj.project !== "string") return false;
  if (!("payload" in obj) || typeof obj.payload !== "string") return false;
  // if (!("origin" in obj) || typeof obj.origin !== "string") return false;
  return true;
}

export type MemQuery = {
  goal: string;
  project?: string;
  top_n?: number;
};

export type MemUpdateUsedRequest = {
  memid: string;
  correct: number;
  relevant: number;
};

export type MemUpdateRequest = {
  memid: string;
  mem_type: string;
  goal: string;
  project: string;
  payload: string;
  origin: string; // TODO: upgrade to serde_json::Value
};

export function isMemUpdateRequest(obj: unknown): obj is MemUpdateRequest {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("memid" in obj) || typeof obj.memid !== "string") return false;
  if (!("mem_type" in obj) || typeof obj.mem_type !== "string") return false;
  if (!("goal" in obj) || typeof obj.goal !== "string") return false;
  if (!("project" in obj) || typeof obj.project !== "string") return false;
  if (!("payload" in obj) || typeof obj.payload !== "string") return false;
  if (!("origin" in obj) || typeof obj.origin !== "string") return false;
  return true;
}

export type CompressTrajectoryPost = {
  project: string;
  messages: ChatMessages;
};

export type SaveTrajectoryResponse = {
  memid: string;
  trajectory: string;
};

function isSaveTrajectoryResponse(obj: unknown): obj is SaveTrajectoryResponse {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("memid" in obj) || typeof obj.memid !== "string") return false;
  if (!("trajectory" in obj) || typeof obj.trajectory !== "string") {
    return false;
  }
  return true;
}

export const knowledgeApi = createApi({
  reducerPath: "knowledgeApi",
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
    createNewMemoryFromMessages: builder.mutation<
      SaveTrajectoryResponse,
      CompressTrajectoryPost
    >({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const messagesForLsp = formatMessagesForLsp(arg.messages);

        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${KNOWLEDGE_CREATE_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: { project: arg.project, messages: messagesForLsp },
        });

        if (response.error) {
          return { error: response.error };
        }

        if (!isSaveTrajectoryResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Invalid response from ${url}`,
              data: response.data,
            },
          };
        }

        return { data: response.data };
      },
    }),

    compressMessages: builder.mutation<
      { goal: string; trajectory: string },
      CompressTrajectoryPost
    >({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const messagesForLsp = formatMessagesForLsp(arg.messages);

        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${COMPRESS_MESSAGES_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: { project: arg.project, messages: messagesForLsp },
        });

        if (response.error) {
          return { error: response.error };
        }

        if (!isCompressMessagesResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Invalid response from ${url}`,
              data: response.data,
            },
          };
        }

        return { data: response.data };
      },
    }),

    setActiveWorkspaceId: builder.mutation<
      unknown,
      { workspace_id: Workspace["workspace_id"] }
    >({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${SET_ACTIVE_WORKSPACE_ID}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: JSON.stringify(arg),
        });

        if (response.error) {
          return { error: response.error };
        }

        if (isDetailMessage(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Error: ${response.data.detail}`,
              data: response.data,
            },
          };
        }

        if (!isSuccess(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Invalid response from ${url}`,
              data: response.data,
            },
          };
        }

        return { data: response.data };
      },
    }),
  }),
});

type CompressMessagesResponse = {
  goal: string;
  trajectory: string;
};

function isCompressMessagesResponse(
  data: unknown,
): data is CompressMessagesResponse {
  if (!data) return false;
  if (typeof data !== "object") return false;
  if (!("goal" in data) || typeof data.goal !== "string") return false;
  if (!("trajectory" in data) || typeof data.trajectory !== "string")
    return false;
  return true;
}

function isSuccess(data: unknown): data is { success: true } {
  return (
    typeof data === "object" &&
    data !== null &&
    "success" in data &&
    typeof data.success === "boolean" &&
    data.success
  );
}
