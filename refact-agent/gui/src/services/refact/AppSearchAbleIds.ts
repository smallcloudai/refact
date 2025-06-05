import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";

// http://localhost:8001/v1/get-app-searchable-id

type GetAppSearchableIdResponse = {
  app_searchable_id: string;
};

function isGetAppSearchableResponse(
  response: unknown,
): response is GetAppSearchableIdResponse {
  if (!response) return false;
  if (typeof response !== "object") return false;
  if (!("app_searchable_id" in response)) return false;
  return typeof response.app_searchable_id === "string";
}
export const appSearchableIdsApi = createApi({
  reducerPath: "searchableId",
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
    getAppSearchableId: builder.mutation<
      { app_searchable_id: string },
      undefined
    >({
      async queryFn(_arg, api, _extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}/v1/get-app-searchable-id`;
        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
        });

        if (result.error) {
          return { error: result.error };
        }

        if (!isGetAppSearchableResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: `Invalid response from ${url}`,
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }
        return { data: result.data };
      },
    }),
  }),
});
