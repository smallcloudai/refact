import { RootState } from "../../app/store";
import { PING_URL } from "./consts";
import {
  createApi,
  fetchBaseQuery,
  FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";

export const pingApi = createApi({
  reducerPath: "pingApi",
  baseQuery: fetchBaseQuery({ baseUrl: PING_URL }),
  tagTypes: ["PING"],
  endpoints: (builder) => ({
    ping: builder.query<string, undefined>({
      providesTags: () => ["PING"],
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        const port = (api.getState() as RootState).config.lspPort;
        const url = `http://127.0.0.1:${port}${PING_URL}`;
        return new Promise((resolve, _reject) => {
          const poll = () => {
            fetch(url, {
              method: "GET",
              redirect: "follow",
              cache: "no-cache",
            })
              .then((res) => {
                if (res.ok) return res.text();
                throw new Error(res.statusText);
              })
              .then((pong) => {
                resolve({ data: pong });
              })
              .catch((err: Error) => {
                if (err.message === "Failed to fetch") {
                  setTimeout(poll, 1000);
                } else {
                  const result: FetchBaseQueryError = {
                    status: "FETCH_ERROR",
                    error: err.message,
                  };

                  resolve({ error: result });
                }
              });
          };
          poll();
        });
      },
    }),
    reset: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: ["PING"],
    }),
  }),
});
