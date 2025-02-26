import { RootState } from "../../app/store";
import { CONFIG_PATH_URL, FULL_PATH_URL } from "./consts";
import {
  BaseQueryApi,
  createApi,
  fetchBaseQuery,
  FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";
import { callEngine } from "./call_engine";

type FullPathResponse = {
  fullpath: string;
  is_directory: boolean;
};

// Reusable function to fetch paths
async function fetchPath(
  api: BaseQueryApi,
  configPathUrl: string,
  suffix: string,
): Promise<{ data: string } | { error: FetchBaseQueryError }> {
  try {
    const state = api.getState() as RootState;
    const response = await callEngine<string>(state, configPathUrl, {
      method: "GET",
      credentials: "same-origin",
      redirect: "follow",
    });

    if (typeof response !== "string") {
      return {
        error: {
          error: `${suffix} path response not a string`,
          status: "CUSTOM_ERROR",
        },
      };
    }
    return { data: response + suffix };
  } catch (error) {
    return {
      error: {
        status: "FETCH_ERROR",
        error: String(error),
      },
    };
  }
}

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
      queryFn: async (path, api, _opts, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, FULL_PATH_URL, {
            credentials: "same-origin",
            redirect: "follow",
            method: "POST",
            body: JSON.stringify({ path }),
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isFullPathResponse(data)) {
            return {
              error: {
                error: "Invalid response from fullpath",
                data: data,
                status: "CUSTOM_ERROR",
              },
            };
          }

          if (data.is_directory) {
            return { data: null };
          }

          return { data: data.fullpath };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
    customizationPath: builder.query<string, undefined>({
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        return await fetchPath(
          api,
          CONFIG_PATH_URL,
          "/customization.yaml",
        );
      },
    }),
    privacyPath: builder.query<string, undefined>({
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        return await fetchPath(
          api,
          CONFIG_PATH_URL,
          "/privacy.yaml",
        );
      },
    }),
    bringYourOwnKeyPath: builder.query<string, undefined>({
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        return await fetchPath(
          api,
          CONFIG_PATH_URL,
          "/bring-your-own-key.yaml",
        );
      },
    }),
    integrationsPath: builder.query<string, undefined>({
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        return await fetchPath(
          api,
          CONFIG_PATH_URL,
          "/integrations.yaml",
        );
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