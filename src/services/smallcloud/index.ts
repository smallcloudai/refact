import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";

export type User = {
  retcode: string;
  account: string;
  inference_url: string;
  inference: string;
  metering_balance: number;
  questionnaire: false | Record<string, string>;
  refact_agent_max_request_num: number;
  refact_agent_request_available: null | number; // null for PRO or ROBOT
};

function isUser(json: unknown): json is User {
  return (
    typeof json === "object" &&
    json !== null &&
    "retcode" in json &&
    typeof json.retcode === "string" &&
    "account" in json &&
    typeof json.account === "string" &&
    "inference_url" in json &&
    typeof json.inference_url === "string" &&
    "inference" in json &&
    typeof json.inference === "string" &&
    "refact_agent_max_request_num" in json &&
    typeof json.refact_agent_max_request_num === "number" &&
    "refact_agent_request_available" in json &&
    (json.refact_agent_request_available === null ||
      typeof json.refact_agent_max_request_num === "number")
  );
}

type GoodResponse = User & {
  secret_key: string;
  tooltip_message: string;
  login_message: string;
  "longthink-filters": unknown[];
  "longthink-functions-today": Record<string, LongThinkFunction>;
  "longthink-functions-today-v2": Record<string, LongThinkFunction>;
};

export function isGoodResponse(json: unknown): json is GoodResponse {
  if (!isUser(json)) return false;
  return "secret_key" in json && typeof json.secret_key === "string";
}

type BadResponse = {
  human_readable_message: string;
  retcode: "FAILED";
};

export type StreamedLoginResponse = GoodResponse | BadResponse;

export type LongThinkFunction = {
  label: string;
  model: string;
  selected_lines_min: number;
  selected_lines_max: number;
  metering: number;
  "3rd_party": boolean;
  supports_highlight: boolean;
  supports_selection: boolean;
  always_visible: boolean;
  mini_html: string;
  likes: number;
  supports_languages: string;
  is_liked: boolean;
  function_highlight: string;
  function_selection: string;
};

export type RadioOptions = {
  title: string;
  value: string;
};

export interface SurveyQuestion {
  type: string;
  name: string;
  question: string;
}

function isSurveyQuestion(json: unknown): json is SurveyQuestion {
  if (!json) return false;
  if (typeof json !== "object") return false;
  return (
    "type" in json &&
    typeof json.type === "string" &&
    "name" in json &&
    typeof json.name === "string" &&
    "question" in json &&
    typeof json.question === "string"
  );
}

export interface RadioQuestion extends SurveyQuestion {
  type: "radio";
  options: RadioOptions[];
}

export function isRadioQuestion(
  question: SurveyQuestion,
): question is RadioQuestion {
  return question.type === "radio";
}

export type SurveyQuestions = (RadioQuestion | SurveyQuestion)[];

function isSurveyQuestions(json: unknown): json is SurveyQuestions {
  if (!Array.isArray(json)) return false;
  return json.every(isSurveyQuestion);
}

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
  }),
});
