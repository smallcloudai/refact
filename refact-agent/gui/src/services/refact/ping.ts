import { PING_URL } from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export const pingApi = createApi({
  reducerPath: "pingApi",
  baseQuery: fetchBaseQuery({ baseUrl: PING_URL }),
  tagTypes: ["PING"],
  endpoints: (builder) => ({
    ping: builder.query<string, number>({
      providesTags: (_result, _error, port) => [{ type: "PING", id: port }],
      forceRefetch: ({ currentArg, previousArg }) => currentArg !== previousArg,
      queryFn: async (portArg, _api, _extraOptions, _baseQuery) => {
        const url = `http://127.0.0.1:${portArg}${PING_URL}`;

        try {
          const response = await fetch(url, {
            method: "GET",
            redirect: "follow",
            cache: "no-cache",
          });

          if (response.ok) {
            const text = await response.text();
            return { data: text };
          } else {
            return {
              error: {
                status: "FETCH_ERROR",
                error: response.statusText,
              },
            };
          }
        } catch (err) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: err instanceof Error ? err.message : String(err),
            },
          };
        }
      },
    }),
    reset: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: [{ type: "PING", id: undefined }],
    }),
  }),
});
