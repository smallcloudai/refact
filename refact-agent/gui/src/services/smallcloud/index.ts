import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";
import {
  EmailLinkResponse,
  isEmailLinkResponse,
  isGoodResponse,
  isSurveyQuestions,
  isUser,
  SurveyQuestions,
  User,
} from "./types";

export const smallCloudApi = createApi({
  reducerPath: "smallcloud",
  baseQuery: fetchBaseQuery({
    baseUrl: "https://www.smallcloud.ai/v1",
    prepareHeaders: (headers, api) => {
      const getState = api.getState as () => RootState;
      const state = getState();
      const token = state.config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  tagTypes: ["User", "Polling"],
  endpoints: (builder) => ({
    login: builder.query({
      providesTags: ["Polling"],
      queryFn: async (token, api, _extraOptions, baseQuery) => {
        return new Promise<ReturnType<typeof baseQuery>>((resolve, reject) => {
          const timeout = setInterval(() => {
            fetch(
              "https://www.smallcloud.ai/v1/streamlined-login-recall-ticket",
              {
                method: "GET",
                headers: {
                  Authorization: `codify-${token}`,
                  "Content-Type": "application/json",
                },
                redirect: "follow",
                cache: "no-cache",
                referrer: "no-referrer",
                signal: api.signal,
              },
            )
              .then((response) => {
                if (!response.ok) {
                  throw new Error(
                    "Invalid response from server: " + response.statusText,
                  );
                }
                return response.json() as unknown;
              })
              .then((json: unknown) => {
                if (isGoodResponse(json)) {
                  clearInterval(timeout);
                  resolve({ data: json });
                }
              })
              .catch((err: Error) => reject(err));
          }, 5000);
        });
      },
    }),
    getUser: builder.query<
      User,
      {
        apiKey: string;
        addressURL?: string;
      }
    >({
      query: (args) => {
        const { apiKey } = args;
        return {
          url: "login",
          method: "GET",
          redirect: "follow",
          cache: "no-cache",
          // referrer: "no-referrer",
          headers: {
            "Content-Type": "application/json",
            Authorization: "Bearer " + apiKey,
          },
        };
      },
      transformResponse(response: unknown) {
        if (!isUser(response)) {
          throw new Error("Invalid response from server");
        }

        return response;
      },
      providesTags: ["User"],
    }),

    getSurvey: builder.query<SurveyQuestions, undefined>({
      query: () => "/questionnaire",
      transformResponse(baseQueryReturnValue, _meta, _arg) {
        if (!isSurveyQuestions(baseQueryReturnValue)) {
          // eslint-disable-next-line no-console
          console.error(baseQueryReturnValue);
          throw new Error("Invalid response from server");
        }
        return baseQueryReturnValue;
      },
    }),

    postSurvey: builder.mutation<null, Record<string, FormDataEntryValue>>({
      query: (arg) => {
        return {
          url: "/save-questionnaire",
          method: "POST",
          body: { questionnaire: arg },
          headers: {
            "Content-Type": "application/json",
          },
        };
      },
      invalidatesTags: ["User"],
    }),

    removeUserFromCache: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: ["User", "Polling"],
    }),

    loginWithEmailLink: builder.mutation<
      EmailLinkResponse,
      { email: string; token: string }
    >({
      async queryFn(arg, api, extraOptions, baseQuery) {
        // TODO: maybe use cookies?
        const url = `https://www.smallcloud.ai/plugin-magic-link/${arg.token.trim()}/${arg.email.trim()}`;

        const response = await baseQuery({
          ...extraOptions,
          url,
          signal: api.signal,
        });
        if (response.error) return response;

        if (!isEmailLinkResponse(response.data)) {
          return {
            error: {
              error:
                "Invalid response from https://www.smallcloud.ai/plugin-magic-link",
              data: response.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: response.data };
      },
    }),
  }),
});
