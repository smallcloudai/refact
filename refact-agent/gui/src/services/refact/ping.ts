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
      queryFn: async (portArg, _api, _extraOptions, baseQuery) => {
        const url = `http://127.0.0.1:${portArg}${PING_URL}`;

        const response = await baseQuery({
          method: "GET",
          url,
          redirect: "follow",
          cache: "no-cache",
          responseHandler: "text",
        });

        if (response.error) {
          return {
            error: response.error,
          };
        }

        if (response.data && typeof response.data === "string") {
          return { data: response.data };
        } else {
          return {
            error: {
              status: "FETCH_ERROR",
              error: "No data received in response",
            },
          };
        }
      },
    }),
    reset: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: ["PING"],
    }),
  }),
});
