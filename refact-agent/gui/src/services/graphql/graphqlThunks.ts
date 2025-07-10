import { createGraphqlClient, createSubscription } from "./createClient";
import { createAsyncThunk } from "@reduxjs/toolkit";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

import {
  ThreadsPageSubsDocument,
  ThreadsPageSubsSubscription,
  ThreadsPageSubsSubscriptionVariables,
  DeleteThreadDocument,
  DeleteThreadMutationVariables,
  DeleteThreadMutation,
  CreateThreadMutation,
  CreateThreadDocument,
  CreateThreadMutationVariables,
  MessagesSubscriptionSubscriptionVariables,
  MessagesSubscriptionDocument,
  MessageCreateMultipleDocument,
  MessageCreateMultipleMutation,
  MessageCreateMultipleMutationVariables,
  MessagesSubscriptionSubscription,
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
  ThreadConfirmationResponseMutation,
  ThreadConfirmationResponseMutationVariables,
  ThreadConfirmationResponseDocument,
  BasicStuffQuery,
  BasicStuffQueryVariables,
  BasicStuffDocument,
  CreateWorkSpaceGroupMutation,
  CreateWorkSpaceGroupMutationVariables,
  CreateWorkSpaceGroupDocument,
  WorkspaceTreeSubscription,
  WorkspaceTreeSubscriptionVariables,
  WorkspaceTreeDocument,
} from "../../../generated/documents";
import { handleThreadListSubscriptionData } from "../../features/ThreadList";
import { setError } from "../../features/Errors/errorsSlice";
import { AppDispatch, RootState } from "../../app/store";
import {
  receiveDeltaStream,
  receiveThread,
  receiveThreadMessages,
  removeMessage,
  setThreadFtId,
} from "../../features/ThreadMessages";
import { Tool } from "../refact/tools";
import { IntegrationMeta } from "../../features/Chat";
import { receiveWorkspace, receiveWorkspaceError } from "../../features/Groups";
import { v4 as uuidv4 } from "uuid";

import { connected, connecting, closed } from "../../features/ConnectionStatus";

export const threadsPageSub = createAsyncThunk<
  unknown,
  ThreadsPageSubsSubscriptionVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
  }
>("graphql/threadsPageSub", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const address = state.config.addressURL ?? "https://app.refact.ai";

  const connectionId = uuidv4();

  createSubscription<
    ThreadsPageSubsSubscription,
    ThreadsPageSubsSubscriptionVariables
  >(
    address,
    apiKey,
    ThreadsPageSubsDocument,
    args,
    thunkAPI.signal,
    (result) => {
      if (result.data) {
        thunkAPI.dispatch(handleThreadListSubscriptionData(result.data));
      } else if (result.error) {
        thunkAPI.dispatch(setError(result.error.message));
      }
    },

    {
      connecting: () => {
        thunkAPI.dispatch(
          connecting({ id: connectionId, name: "ThreadsPageSub" }),
        );
      },

      connected: () => {
        thunkAPI.dispatch(
          connected({ id: connectionId, name: "ThreadsPageSub" }),
        );
      },

      closed: () => {
        thunkAPI.dispatch(closed({ id: connectionId }));
      },
    },
  );
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";
  const address = state.config.addressURL ?? "https://app.refact.ai";

  const connectionId = uuidv4();

  const sub = createSubscription<
    MessagesSubscriptionSubscription,
    MessagesSubscriptionSubscriptionVariables
  >(
    address,
    apiKey,
    MessagesSubscriptionDocument,
    args,
    thunkApi.signal,
    (result) => {
      if (thunkApi.signal.aborted) {
        // eslint-disable-next-line no-console
        console.log("handleResult called after thunk signal is aborted");

        return thunkApi.fulfillWithValue({});
      }
      if (result.error) {
        // TBD: do we hang up on errors?
        thunkApi.dispatch(setError(result.error.message));
      }

      if (result.data?.comprehensive_thread_subs.news_payload_thread) {
        thunkApi.dispatch(
          receiveThread({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            news_payload_thread:
              result.data.comprehensive_thread_subs.news_payload_thread,
          }),
        );
      }
      if (result.data?.comprehensive_thread_subs.stream_delta) {
        thunkApi.dispatch(
          receiveDeltaStream({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            stream_delta: result.data.comprehensive_thread_subs.stream_delta,
          }),
        );
      }

      if (result.data?.comprehensive_thread_subs.news_action === "DELETE") {
        thunkApi.dispatch(
          removeMessage({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
          }),
        );
      }

      if (result.data?.comprehensive_thread_subs.news_payload_thread_message) {
        thunkApi.dispatch(
          receiveThreadMessages({
            news_action: result.data.comprehensive_thread_subs.news_action,
            news_payload_id:
              result.data.comprehensive_thread_subs.news_payload_id,
            news_payload_thread_message:
              result.data.comprehensive_thread_subs.news_payload_thread_message,
          }),
        );
      }
    },
    {
      connecting: () => {
        thunkApi.dispatch(
          connecting({ id: connectionId, name: "MessagesSub" }),
        );
      },

      connected: () => {
        thunkApi.dispatch(connected({ id: connectionId, name: "MessagesSub" }));
      },

      closed: () => {
        thunkApi.dispatch(closed({ id: connectionId }));
      },
    },
  );

  // TODO: duplicated call to unsubscribe
  thunkApi.signal.addEventListener("abort", () => {
    sub.unsubscribe();
    thunkApi.fulfillWithValue({});
  });

  // return thunkApi.fulfillWithValue({});
});

export function rejectToolUsageAction(
  ids: string[],
  ft_id: string,
  endNumber: number,
  endAlt: number,
  endPrevAlt: number,
) {
  const messagesToSend: FThreadMessageInput[] = ids.map((id, index) => {
    return {
      ftm_role: "tool",
      ftm_belongs_to_ft_id: ft_id,
      ftm_content: JSON.stringify("The user rejected the changes."),
      ftm_call_id: id,
      ftm_num: endNumber + index + 1,
      ftm_alt: endAlt,
      ftm_prev_alt: endPrevAlt,
      ftm_provenance: "null",
    };
  });

  const action = graphqlQueriesAndMutations.endpoints.sendMessages.initiate({
    input: { messages: messagesToSend, ftm_belongs_to_ft_id: ft_id },
  });

  return action;
}

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

// TODO: stop is ft_error, set this and it'll stop

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

// TODO: move to queries and mutations api
export const pauseThreadThunk = createAsyncThunk<
  ThreadPatchMutation,
  { id: string },
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("flexus/thread/pause", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);

  const result = await client.mutation<
    ThreadPatchMutation,
    ThreadPatchMutationVariables
  >(ThreadPatchDocument, { id: args.id, message: JSON.stringify("pause") });

  if (result.error) {
    return thunkAPI.rejectWithValue(result.error);
  }

  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "failed to stop thread" });
  }
  return thunkAPI.fulfillWithValue(result.data);
});

// TODO: move to queries and mutations api
export const toolConfirmationThunk = createAsyncThunk<
  ThreadConfirmationResponseMutation,
  ThreadConfirmationResponseMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: {
      message: string;
      args: ThreadConfirmationResponseMutationVariables;
    };
  }
>("flexus/tools/confirmation/response", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);
  const result = await client.mutation<
    ThreadConfirmationResponseMutation,
    ThreadConfirmationResponseMutationVariables
  >(ThreadConfirmationResponseDocument, args);

  if (result.error) {
    return thunkAPI.rejectWithValue({ message: result.error.message, args });
  } else if (!result.data) {
    return thunkAPI.rejectWithValue({
      message: "failed to confirm tools",
      args,
    });
  }

  return thunkAPI.fulfillWithValue(result.data);
});

export const workspaceTreeSubscriptionThunk = createAsyncThunk<
  unknown,
  WorkspaceTreeSubscriptionVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: {
      message: string;
      args: WorkspaceTreeSubscriptionVariables;
    };
  }
>("flexus/treeSubscription", (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;
  const connectionId = uuidv4();
  createSubscription<
    WorkspaceTreeSubscription,
    WorkspaceTreeSubscriptionVariables
  >(
    addressUrl,
    apiKey,
    WorkspaceTreeDocument,
    args,
    thunkAPI.signal,
    (result) => {
      if (result.error) {
        thunkAPI.dispatch(receiveWorkspaceError(result.error.message));
      }
      if (result.data) {
        thunkAPI.dispatch(receiveWorkspace(result.data.tree_subscription));
      }
    },
    {
      connecting: () => {
        thunkAPI.dispatch(
          connecting({ id: connectionId, name: "WorkspaceTreeSub" }),
        );
      },

      connected: () => {
        thunkAPI.dispatch(
          connected({ id: connectionId, name: "WorkSpaceTreeSub" }),
        );
      },

      closed: () => {
        thunkAPI.dispatch(closed({ id: connectionId }));
      },
    },
  );
});

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
        // TODO: where is current workspace set?
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
              ftm_app_specific: JSON.stringify(
                appIdQuery.data?.app_searchable_id ?? "",
              ),
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
                tools: args.tools,
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
        content: string;
        expertId: string;
        model: string;
        tools: (Tool["spec"] | FCloudTool)[];
      }
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        const state = api.getState() as RootState;
        const apiKey = state.config.apiKey ?? "";
        const port = state.config.lspPort;
        // TODO: where is current workspace set?
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
          ftm_app_specific: JSON.stringify(
            appIdQuery.data?.app_searchable_id ?? "",
          ),
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
            tools: args.tools,
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
  }),
});
