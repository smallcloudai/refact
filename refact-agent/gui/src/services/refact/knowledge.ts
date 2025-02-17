import { AppDispatch, RootState } from "../../app/store";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import {
  consumeStream,
  formatMessagesForLsp,
} from "../../features/Chat/Thread/utils";
import {
  KNOWLEDGE_ADD_URL,
  KNOWLEDGE_CREATE_URL,
  KNOWLEDGE_REMOVE_URL,
  KNOWLEDGE_SUB_URL,
  KNOWLEDGE_UPDATE_URL,
  KNOWLEDGE_UPDATE_USED_URL,
} from "./consts";
import type { ChatMessages } from ".";
import { parseOrElse } from "../../utils";
import { createAsyncThunk } from "@reduxjs/toolkit/react";
import { type MemoRecord, isMemoRecord, isVecDbStatus } from "./types";
import {
  clearMemory,
  deleteMemory,
  setMemory,
  setVecDbStatus,
} from "../../features/Knowledge/knowledgeSlice";

export type MemdbSubEvent = {
  pubevent_id: number;
  pubevent_action: string;
  pubevent_json: MemoRecord;
};

function isMemdbSubEvent(obj: unknown): obj is MemdbSubEvent {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("pubevent_id" in obj) || typeof obj.pubevent_id !== "number") {
    return false;
  }
  if (!("pubevent_action" in obj) || typeof obj.pubevent_action !== "string") {
    return false;
  }
  if (!("pubevent_json" in obj) || !isMemoRecord(obj.pubevent_json)) {
    return false;
  }
  return true;
}

export type MemdbSubEventUnparsed = {
  pubevent_id: number;
  pubevent_action: string;
  pubevent_json: string;
};

function isMemdbSubEventUnparsed(obj: unknown): obj is MemdbSubEventUnparsed {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("pubevent_id" in obj) || typeof obj.pubevent_id !== "number") {
    return false;
  }
  if (!("pubevent_action" in obj) || typeof obj.pubevent_action !== "string") {
    return false;
  }
  if (!("pubevent_json" in obj) || typeof obj.pubevent_json !== "string") {
    return false;
  }
  return true;
}

export type SubscribeArgs =
  | {
      quick_search?: string;
      limit?: number;
    }
  | undefined;

function subscribeToMemories(
  port = 8001,
  args: SubscribeArgs,
  apiKey?: string | null,
  abortSignal?: AbortSignal,
): Promise<Response> {
  const url = `http://127.0.0.1:${port}${KNOWLEDGE_SUB_URL}`;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");
  if (apiKey) {
    headers.append("Authorization", `Bearer ${apiKey}`);
  }

  return fetch(url, {
    method: "POST",
    headers,
    redirect: "follow",
    cache: "no-cache",
    body: args ? JSON.stringify(args) : undefined,
    signal: abortSignal,
  });
}

const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();
// use this
export const subscribeToMemoriesThunk = createAppAsyncThunk<
  unknown,
  SubscribeArgs
>("knowledge/subscription", (args, thunkApi) => {
  const state = thunkApi.getState() as unknown as RootState;
  const port = state.config.lspPort;
  const apiKey = state.config.apiKey;

  return subscribeToMemories(port, args, apiKey, thunkApi.signal)
    .then((response) => {
      if (!response.ok) {
        throw new Error(response.statusText);
      }
      const reader = response.body?.getReader();
      if (!reader) return;
      thunkApi.dispatch(clearMemory());
      const onAbort = () => {
        // console.log("knowledge stream aborted");
      };
      const onChunk = (chunk: Record<string, unknown>) => {
        if (
          !isMemdbSubEvent(chunk) &&
          !isMemdbSubEventUnparsed(chunk) &&
          !isVecDbStatus(chunk)
        ) {
          // eslint-disable-next-line no-console
          console.log("Invalid chunk from mem db", chunk);
          return;
        }

        if (isVecDbStatus(chunk)) {
          const action = setVecDbStatus(chunk);
          thunkApi.dispatch(action);
          return;
        }
        const maybeMemoRecord: MemoRecord | null = isMemoRecord(
          chunk.pubevent_json,
        )
          ? chunk.pubevent_json
          : parseOrElse(chunk.pubevent_json, null, isMemoRecord);

        if (maybeMemoRecord === null) {
          return;
        }

        if (chunk.pubevent_action === "DELETE") {
          const action = deleteMemory(maybeMemoRecord.memid);
          thunkApi.dispatch(action);
        } else if (
          chunk.pubevent_action === "INSERT" ||
          chunk.pubevent_action === "UPDATE"
        ) {
          const action = setMemory(maybeMemoRecord);
          thunkApi.dispatch(action);
        } else {
          // eslint-disable-next-line no-console
          console.log("Unknown action", chunk.pubevent_action);
        }
      };

      return consumeStream(reader, thunkApi.signal, onAbort, onChunk);
    })
    .catch((err) => {
      // eslint-disable-next-line no-console
      console.error("Error in memory subscription", err);
    });
});

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

type MemAddResponse = {
  memid: string;
};
function isMemAddResponse(obj: unknown): obj is MemAddResponse {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (!("memid" in obj) || typeof obj.memid !== "string") return false;
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

export type CompressTrajectoryResponse = {
  memid: string;
  trajectory: string;
};

function isCompressTrajectoryResponse(
  obj: unknown,
): obj is CompressTrajectoryResponse {
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
    addMemory: builder.mutation<MemAddResponse, MemAddRequest>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${KNOWLEDGE_ADD_URL}`;

        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: {
            mem_type: "",
            origin: "",
            project: "",
            ...arg,
          },
        });

        if (response.error) {
          return response;
        }

        if (!isMemAddResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Invalid response from ${url}`,
              data: response.data,
            },
            meta: response.meta,
          };
        }

        return { data: response.data, meta: response.meta };
      },
    }),

    deleteMemory: builder.mutation<unknown, string>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${KNOWLEDGE_REMOVE_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: { memid: arg },
        });
        return response;
      },
    }),

    updateMemory: builder.mutation<unknown, MemUpdateRequest>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${KNOWLEDGE_UPDATE_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: arg,
        });
        return response;
      },
    }),

    updateMemoryUsage: builder.mutation<unknown, MemUpdateUsedRequest>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${KNOWLEDGE_UPDATE_USED_URL}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: arg,
        });
        return response;
      },
    }),

    createNewMemoryFromMessages: builder.mutation<
      CompressTrajectoryResponse,
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

        if (!isCompressTrajectoryResponse(response.data)) {
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
