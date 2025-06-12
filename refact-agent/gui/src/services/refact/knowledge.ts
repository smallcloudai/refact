import { RootState } from "../../app/store";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { formatMessagesForLsp } from "../../features/Chat/Thread/utils";
import { COMPRESS_MESSAGES_URL } from "./consts";
import { type ChatMessages } from ".";

export type CompressTrajectoryPost = {
  project: string;
  messages: ChatMessages;
};

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
