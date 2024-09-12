import { RootState } from "../../app/store";
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
        const url = `http://127.0.0.1:${port}/v1/fullpath`;
        // return baseQuery(url);
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

        console.log({ result, path });

        if (!isFullPathResponse(result.data)) {
          console.log("Invalid");
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
          console.log("isDirectory");
          return { data: null };
        }

        console.log("Got data");
        console.log(result.data);

        return { data: result.data.fullpath };
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
