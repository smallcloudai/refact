import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { usePostMessage } from "../../hooks";
import { EVENT_NAMES_FROM_SETUP, OpenExternalUrl } from "../../events/setup";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import {
  selectApiKey,
  selectHost,
  setApiKey,
} from "../../features/Config/configSlice";
import { useCallback, useEffect, useMemo, useState } from "react";

//https://redux-toolkit.js.org/rtk-query/usage/polling

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

// TODO: move these to the hooks directory.
export const useGetUser = () => {
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  return smallCloudApi.useGetUserQuery(apiKey, { skip: !maybeApiKey });
};

const useLoginPolling = (skip: boolean) => {
  const host = useAppSelector(selectHost);

  const newLoginTicket = useMemo(() => {
    return (
      Math.random().toString(36).substring(2, 15) +
      "-" +
      Math.random().toString(36).substring(2, 15)
    );
  }, []);

  const result = smallCloudApi.useLoginQuery(newLoginTicket, {
    skip: skip,
    // refetchOnMountOrArgChange: true,
    // skipPollingIfUnfocused: true,
  });

  useEffect(() => {
    if (!skip) {
      const initUrl = new URL("https://refact.smallcloud.ai/authentication");
      initUrl.searchParams.set("token", newLoginTicket);
      initUrl.searchParams.set("utm_source", "plugin");
      initUrl.searchParams.set("utm_medium", host);
      initUrl.searchParams.set("utm_campaign", "login");
      const initUrlString = initUrl.toString();
      const openUrlMessage: OpenExternalUrl = {
        type: EVENT_NAMES_FROM_SETUP.OPEN_EXTERNAL_URL,
        payload: { url: initUrlString },
      };
      postMessage(openUrlMessage);
    }
  }, [host, newLoginTicket, skip]);

  return result;
};

export const useLogin = () => {
  const dispatch = useAppDispatch();
  const user = useGetUser();
  const logout = useLogout();

  const [isPollingLogin, setIsPollingLogin] = useState<boolean>(false);
  const canLogin = !user.data && !isPollingLogin;
  const loginPollingResult = useLoginPolling(canLogin);

  const loginThroughWeb = useCallback(() => {
    setIsPollingLogin(true);
  }, []);

  // TODO: handle errors
  const loginWithKey = useCallback(
    (key: string) => {
      setIsPollingLogin(false);
      dispatch(setApiKey(key));
    },
    [dispatch],
  );

  useEffect(() => {
    if (isGoodResponse(loginPollingResult.data)) {
      setIsPollingLogin(false);
      dispatch(setApiKey(loginPollingResult.data.secret_key));
    }
  }, [dispatch, loginPollingResult.data]);

  return {
    loginThroughWeb,
    loginWithKey,
    user,
    polling: loginPollingResult,
    logout,
  };
};

const useLogout = () => {
  const postMessage = usePostMessage();
  const dispatch = useAppDispatch();

  const logout = useCallback(() => {
    postMessage({ type: EVENT_NAMES_FROM_SETUP.LOG_OUT });
    dispatch(setApiKey(null));
    dispatch(smallCloudApi.util.invalidateTags(["User", "Polling"]));
  }, [dispatch, postMessage]);

  return logout;
};
