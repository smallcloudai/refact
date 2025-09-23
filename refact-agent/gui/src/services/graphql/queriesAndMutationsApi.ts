import { createGraphqlClient } from "./createClient";

import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import {
  DeleteThreadDocument,
  DeleteThreadMutationVariables,
  DeleteThreadMutation,
  CreateThreadMutation,
  CreateThreadDocument,
  CreateThreadMutationVariables,
  MessageCreateMultipleDocument,
  MessageCreateMultipleMutation,
  MessageCreateMultipleMutationVariables,
  FThreadMessageInput,
  FThreadInput,
  ThreadPatchDocument,
  ThreadPatchMutation,
  ThreadPatchMutationVariables,
  ExpertsForGroupDocument,
  ExpertsForGroupQuery,
  ExpertsForGroupQueryVariables,
  ModelsForExpertDocument,
  ModelsForExpertQuery,
  ModelsForExpertQueryVariables,
  ToolsForGroupQuery,
  ToolsForGroupQueryVariables,
  ToolsForGroupDocument,
  FCloudTool,
  BasicStuffQuery,
  BasicStuffQueryVariables,
  BasicStuffDocument,
  CreateWorkSpaceGroupMutation,
  CreateWorkSpaceGroupMutationVariables,
  CreateWorkSpaceGroupDocument,
  ThreadConfirmationResolveDocument,
  ThreadConfirmationResolveMutation,
  ThreadConfirmationResolveMutationVariables,
} from "../../../generated/documents";

import { type RootState } from "../../app/store";
import { setThreadFtId } from "../../features/ThreadMessages";
import { Tool } from "../refact/tools";
import type { IntegrationMeta } from "../../features/ThreadMessages";
import { UserMessage } from "../refact/types";

async function fetchAppSearchableId(apiKey: string, port: number) {
  const appIdUrl = `http://127.0.0.1:${port}/v1/get-app-searchable-id`;
  const appIdQuery = await fetch(appIdUrl, {
    credentials: "same-origin",
    redirect: "follow",
    headers: { Authorization: `Bearer ${apiKey}` },
  })
    .then((res) => res.json())
    .then((json) => {
      if (!isGetAppSearchableResponse(json)) {
        const message = `failed parse get_app_searchable_id response: ${JSON.stringify(
          json,
        )}`;
        return {
          data: null,
          error: message,
        };
      }
      return {
        data: json,
        error: null,
      };
    })
    .catch((error: Error) => ({ error: error, data: null }));

  return appIdQuery;
}

type GetAppSearchableIdResponse = {
  app_searchable_id: string;
};

function isGetAppSearchableResponse(
  response: unknown,
): response is GetAppSearchableIdResponse {
  if (!response) return false;
  if (typeof response !== "object") return false;
  if (!("app_searchable_id" in response)) return false;
  return typeof response.app_searchable_id === "string";
}

// TODO: add more queries and mutations and make a new file
export const graphqlQueriesAndMutations = createApi({
  reducerPath: "graphqlQueriesAndMutations",
  baseQuery: fetchBaseQuery(),
  endpoints: (builder) => ({
    createGroup: builder.mutation<
      CreateWorkSpaceGroupMutation,
      CreateWorkSpaceGroupMutationVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;
        const client = createGraphqlClient(addressUrl, apiKey, api.signal);
        const result = await client.mutation<
          CreateWorkSpaceGroupMutation,
          CreateWorkSpaceGroupMutationVariables
        >(CreateWorkSpaceGroupDocument, args);

        if (result.error ?? !result.data) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: result.error?.message ?? "no response when creating group",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),

    getBasicStuff: builder.query<
      BasicStuffQuery,
      { apiKey: string; addressUrl: string }
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const client = createGraphqlClient(
          args.addressUrl,
          args.apiKey,
          api.signal,
        );

        const result = await client.query<
          BasicStuffQuery,
          BasicStuffQueryVariables
        >(BasicStuffDocument, {});
        // const { operation: _, ...rest } = result;
        // return rest;

        if (result.error ?? !result.data) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error:
                result.error?.message ??
                "no response when fetching for basic_stuff.",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),

    sendMessages: builder.mutation<
      MessageCreateMultipleMutation,
      MessageCreateMultipleMutationVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const result = await client.mutation<
          MessageCreateMultipleMutation,
          MessageCreateMultipleMutationVariables
        >(MessageCreateMultipleDocument, args);

        if (result.error ?? !result.data) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: result.error?.message ?? "No data in response",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),

    createThreadWitMultipleMessages: builder.mutation<
      MessageCreateMultipleMutation & CreateThreadMutation,
      {
        messages: { ftm_role: string; ftm_content: unknown }[];
        expertId: string;
        model: string;
        tools: (Tool["spec"] | FCloudTool)[];
        integration?: IntegrationMeta;
      }
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const port = state.config.lspPort;

        const workspace =
          state.teams.group?.id ?? state.config.currentWorkspaceName ?? "";

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;
        const appIdQuery = await fetchAppSearchableId(apiKey, port);

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const threadQueryArgs: FThreadInput = {
          ft_fexp_id: args.expertId,
          ft_title: "",
          located_fgroup_id: workspace,
          owner_shared: false,
          ft_app_searchable: appIdQuery.data?.app_searchable_id,
          ft_toolset: JSON.stringify(args.tools),
        };
        const threadQuery = await client.mutation<
          CreateThreadMutation,
          CreateThreadMutationVariables
        >(CreateThreadDocument, { input: threadQueryArgs });

        if (threadQuery.error) {
          return {
            error: { error: threadQuery.error.message, status: "FETCH_ERROR" },
          };
        }

        if (!threadQuery.data) {
          return {
            error: {
              error: "no data in response from thread creation ",
              status: "CUSTOM_ERROR",
            },
          };
        }

        if (state.threadMessages.ft_id === null) {
          api.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
        }

        const createMessageArgs: FThreadMessageInput[] = args.messages.map(
          (message, index) => {
            return {
              ftm_belongs_to_ft_id: threadQuery.data?.thread_create.ft_id ?? "",
              ftm_alt: 100,
              ftm_num: index + 1,
              ftm_call_id: "",
              ftm_prev_alt: 100,
              ftm_role: message.ftm_role,
              ftm_content: JSON.stringify(message.ftm_content),
              ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
              ftm_tool_calls: "null", // optional
              ftm_usage: "null", // optional
              ftm_user_preferences: JSON.stringify({
                model: args.model,
                ...(args.integration ? { integration: args.integration } : {}),
              }),
            };
          },
        );

        const result = await client.mutation<
          MessageCreateMultipleMutation,
          MessageCreateMultipleMutationVariables
        >(MessageCreateMultipleDocument, {
          input: {
            ftm_belongs_to_ft_id: threadQuery.data.thread_create.ft_id,
            messages: createMessageArgs,
          },
        });

        if (result.error) {
          return {
            error: { error: result.error.message, status: "FETCH_ERROR" },
          };
        }
        if (!result.data) {
          return {
            error: {
              error: "failed to create message",
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: { ...threadQuery.data, ...result.data } };
      },
    }),

    createThreadWithSingleMessage: builder.mutation<
      MessageCreateMultipleMutation & CreateThreadMutation,
      {
        content: UserMessage["ftm_content"];
        expertId: string;
        model: string;
        tools: (Tool["spec"] | FCloudTool)[];
      }
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const port = state.config.lspPort;

        const workspace =
          state.teams.group?.id ?? state.config.currentWorkspaceName ?? "";

        const appIdQuery = await fetchAppSearchableId(apiKey, port);

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const threadQueryArgs: FThreadInput = {
          ft_fexp_id: args.expertId, // TODO: user selected
          ft_title: "", // TODO: generate the title
          located_fgroup_id: workspace,
          owner_shared: false,
          ft_app_searchable: appIdQuery.data?.app_searchable_id,
          ft_toolset: JSON.stringify(args.tools),
        };
        const threadQuery = await client.mutation<
          CreateThreadMutation,
          CreateThreadMutationVariables
        >(CreateThreadDocument, { input: threadQueryArgs });

        if (threadQuery.error) {
          // return thunkAPI.rejectWithValue({
          //   message: threadQuery.error.message,
          // });
          return {
            error: { error: threadQuery.error.message, status: "FETCH_ERROR" },
          };
        }

        if (!threadQuery.data) {
          // return thunkAPI.rejectWithValue({
          //   message: "couldn't create flexus thread id",
          // });
          return {
            error: {
              error: "couldn't create flexus thread",
              status: "CUSTOM_ERROR",
            },
          };
        }

        if (state.threadMessages.ft_id === null) {
          api.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
        }

        // Note: ftm_num, ftm_alt, and ftm_prev_alt are also hard coded for tracking waiting state
        const createMessageArgs: FThreadMessageInput = {
          ftm_belongs_to_ft_id: threadQuery.data.thread_create.ft_id,
          ftm_alt: 100,
          ftm_num: 1,
          ftm_call_id: "",
          ftm_prev_alt: 100,
          ftm_role: "user",
          ftm_content: JSON.stringify(args.content),
          ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
          ftm_tool_calls: "null", // optional
          ftm_usage: "null", // optional
          ftm_user_preferences: JSON.stringify({
            model: args.model,
          }),
        };

        const result = await client.mutation<
          MessageCreateMultipleMutation,
          MessageCreateMultipleMutationVariables
        >(MessageCreateMultipleDocument, {
          input: {
            ftm_belongs_to_ft_id: threadQuery.data.thread_create.ft_id,
            messages: [createMessageArgs],
          },
        });

        if (result.error) {
          // return thunkAPI.rejectWithValue({ message: result.error.message });
          return {
            error: { error: result.error.message, status: "FETCH_ERROR" },
          };
        }
        if (!result.data) {
          return {
            error: {
              error: "failed to create message",
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: { ...threadQuery.data, ...result.data } };
      },
    }),

    experts: builder.query<ExpertsForGroupQuery, ExpertsForGroupQueryVariables>(
      {
        async queryFn(args, api, _extraOptions, _baseQuery) {
          const state = api.getState() as RootState;
          const apiKey = state.config.apiKey ?? "";

          const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

          const client = createGraphqlClient(addressUrl, apiKey, api.signal);

          const result = await client.query<
            ExpertsForGroupQuery,
            ExpertsForGroupQueryVariables
          >(ExpertsForGroupDocument, args);

          if (result.error) {
            return {
              error: { error: result.error.message, status: "FETCH_ERROR" },
            };
          }
          if (!result.data) {
            return {
              error: {
                error: "failed to get expert data",
                status: "CUSTOM_ERROR",
              },
            };
          }

          return { data: result.data };
        },
      },
    ),
    modelsForExpert: builder.query<
      ModelsForExpertQuery,
      ModelsForExpertQueryVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const result = await client.query<
          ModelsForExpertQuery,
          ModelsForExpertQueryVariables
        >(ModelsForExpertDocument, args);

        if (result.error) {
          return {
            error: { status: "FETCH_ERROR", error: result.error.message },
          };
        }

        if (!result.data) {
          return {
            error: {
              error: `failed to models for expert ${args.fexp_id}`,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),

    toolsForWorkspace: builder.query<
      ToolsForGroupQuery,
      ToolsForGroupQueryVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const addressUrl = state.config.addressURL ?? "https://app.refact.ai";

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const result = await client.query<
          ToolsForGroupQuery,
          ToolsForGroupQueryVariables
        >(ToolsForGroupDocument, args);

        if (result.error) {
          // return thunkAPI.rejectWithValue({
          //   message: result.error.message,
          //   args,
          // });
          return {
            error: { error: result.error.message, status: "FETCH_ERROR" },
          };
        }
        if (!result.data) {
          // return thunkAPI.rejectWithValue({
          //   message: "erro fetching tools",
          //   args,
          // });
          return {
            error: { error: "no data in tool request", status: "CUSTOM_ERROR" },
          };
        }

        return { data: result.data };
      },
    }),

    deleteThread: builder.mutation<
      DeleteThreadMutation,
      DeleteThreadMutationVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);
        const result = await client.mutation<
          DeleteThreadMutation,
          DeleteThreadMutationVariables
        >(DeleteThreadDocument, args);
        if (result.error) {
          return {
            error: {
              error: result.error.message,
              status: "FETCH_ERROR",
            },
          };
        }
        if (!result.data) {
          return {
            error: {
              error: "no response data from thread deletion",
              status: "CUSTOM_ERROR",
            },
          };
        }
        return { data: result.data };
      },
    }),

    pauseThread: builder.mutation<ThreadPatchMutation, { id: string }>({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);

        const result = await client.mutation<
          ThreadPatchMutation,
          ThreadPatchMutationVariables
        >(ThreadPatchDocument, {
          id: args.id,
          message: JSON.stringify("pause"),
        });

        if (result.error) {
          return {
            error: { error: result.error.message, status: "FETCH_ERROR" },
          };
        }
        if (!result.data) {
          return {
            error: { error: "failed to pause thread", status: "CUSTOM_ERROR" },
          };
        }

        return { data: result.data };
      },
    }),

    toolConfirmation: builder.mutation<
      ThreadConfirmationResolveMutation,
      ThreadConfirmationResolveMutationVariables
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";

        const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

        const client = createGraphqlClient(addressUrl, apiKey, api.signal);
        const result = await client.mutation<
          ThreadConfirmationResolveMutation,
          ThreadConfirmationResolveMutationVariables
        >(ThreadConfirmationResolveDocument, args);

        if (result.error) {
          return {
            error: { error: result.error.message, status: "FETCH_ERROR" },
          };
        } else if (!result.data) {
          return {
            error: { error: "failed to confirm tools", status: "CUSTOM_ERROR" },
          };
        }

        return { data: result.data };
      },
    }),
  }),
});
