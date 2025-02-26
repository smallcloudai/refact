import { RootState } from "../../app/store";
import { PING_URL } from "./consts";
import {
  createApi,
  fetchBaseQuery,
  FetchBaseQueryError,
} from "@reduxjs/toolkit/query/react";
import { pollEngine } from "./call_engine";

export const pingApi = createApi({
  reducerPath: "pingApi",
  baseQuery: fetchBaseQuery({ baseUrl: PING_URL }),
  tagTypes: ["PING"],
  endpoints: (builder) => ({
    ping: builder.query<string, undefined>({
      providesTags: () => ["PING"],
      queryFn: async (_arg, api, _extraOptions, _baseQuery) => {
        try {
          const state = api.getState() as RootState;
          const data = await pollEngine<string>(state, PING_URL, {
            method: "GET",
            redirect: "follow",
            cache: "no-cache",
          });
          return { data };
        } catch (error) {
          const result: FetchBaseQueryError = {
            status: "FETCH_ERROR",
            error: String(error),
          };
          return { error: result };
        }
      },
    }),
    reset: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: ["PING"],
    }),
  }),
});