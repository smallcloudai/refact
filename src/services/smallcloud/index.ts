import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export type User = {
  retcode: string;
  account: string;
  inference_url: string;
  inference: string;
};

function isUser(json: unknown): json is StreamedLoginResponse {
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
    typeof json.inference === "string"
  );
}

type GoodResponse = User & {
  secret_key: string;
  tooltip_message: string;
  login_message: string;
  "longthink-filters": unknown[];
  "longthink-functions-today": Record<string, LongThinkFunction>;
  "longthink-functions-today-v2": Record<string, LongThinkFunction>;
  metering_balance: number;
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

export const smallCloudApi = createApi({
  reducerPath: "smallcloud",
  baseQuery: fetchBaseQuery({ baseUrl: "https://www.smallcloud.ai/v1" }),
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
    getUser: builder.query({
      query: (apiKey: string) => {
        return {
          url: "login",
          method: "GET",
          redirect: "follow",
          cache: "no-cache",
          referrer: "no-referrer",
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

    removeUserFromCache: builder.mutation<null, undefined>({
      queryFn: () => ({ data: null }),
      invalidatesTags: ["User", "Polling"],
    }),
  }),
});
