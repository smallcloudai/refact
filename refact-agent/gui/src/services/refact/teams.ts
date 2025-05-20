import { RootState } from "../../app/store";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { SET_ACTIVE_GROUP_ID } from "./consts";
import { isDetailMessage, isSuccess, SuccessResponse } from ".";

export const teamsApi = createApi({
  reducerPath: "teamsApi",
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
    setActiveGroupId: builder.mutation<SuccessResponse, { group_id: number }>({
      async queryFn(arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${SET_ACTIVE_GROUP_ID}`;
        const response = await baseQuery({
          ...extraOptions,
          url,
          method: "POST",
          body: JSON.stringify(arg),
        });

        if (response.error) {
          return { error: response.error };
        }

        if (isDetailMessage(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Error: ${response.data.detail}`,
              data: response.data,
            },
          };
        }

        if (!isSuccess(response.data)) {
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
