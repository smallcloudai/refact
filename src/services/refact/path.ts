import { RootState } from "../../app/store";
import { CONFIG_PATH_URL, FULL_PATH_URL } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

type FullPathResponse = {
  fullpath: string;
  is_directory: boolean;
};

export const pathApi = createApi({
  reducerPath: "pathApi",
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
    getFullPath: builder.query<string | null, string>({
      queryFn: async (path, api, _opts, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${FULL_PATH_URL}`;
        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
          method: "POST",
          body: { path },
        });
        if (result.error) {
          return { error: result.error };
        }

        if (!isFullPathResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from fullpath",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        if (result.data.is_directory) {
          return { data: null };
        }

        return { data: result.data.fullpath };
      },
    }),
    customizationPath: builder.query<string, undefined>({
      queryFn: async (_arg, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const previewEndpoint = `http://127.0.0.1:${port}${CONFIG_PATH_URL}`;
        const response = await baseQuery({
          url: previewEndpoint,
          method: "GET",
          ...extraOptions,
          responseHandler: "text",
        });
        if (response.error) return response;
        if (typeof response.data !== "string") {
          return {
            error: {
              error: "customization path response not a string",
              status: "CUSTOM_ERROR",
              data: response.data,
            },
          };
        }
        return { data: response.data + "/customization.yaml" };
      },
    }),
  }),
});

function isFullPathResponse(x: unknown): x is FullPathResponse {
  if (typeof x !== "object" || x === null) {
    return false;
  }
  if (!("fullpath" in x) || !("is_directory" in x)) {
    return false;
  }
  if (typeof x.fullpath !== "string") {
    return false;
  }
  if (typeof x.is_directory !== "boolean") {
    return false;
  }
  return true;
}
