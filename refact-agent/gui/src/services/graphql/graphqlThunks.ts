import { createGraphqlClient, createSubscription } from "./createClient";
import { createAsyncThunk } from "@reduxjs/toolkit";

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
  );
});

export const deleteThreadThunk = createAsyncThunk<
  { id: string },
  DeleteThreadMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; id: string };
  }
>("graphql/deleteThread", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);
  const result = await client.mutation<
    DeleteThreadMutation,
    DeleteThreadMutationVariables
  >(DeleteThreadDocument, args);
  if (result.error) {
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      id: args.id,
    });
  }
  return thunkAPI.fulfillWithValue({ id: args.id });
});

export const messagesSub = createAsyncThunk<
  unknown,
  MessagesSubscriptionSubscriptionVariables,
  { dispatch: AppDispatch; state: RootState }
>("graphql/messageSubscription", (args, thunkApi) => {
  const state = thunkApi.getState();
  const apiKey = state.config.apiKey ?? "";
  const address = state.config.addressURL ?? "https://app.refact.ai";

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

      // console.log(result);
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
  );

  // TODO: duplicated call to unsubscribe
  thunkApi.signal.addEventListener("abort", () => {
    sub.unsubscribe();
    thunkApi.fulfillWithValue({});
  });

  // return thunkApi.fulfillWithValue({});
});

export const createMessage = createAsyncThunk<
  MessageCreateMultipleMutation,
  MessageCreateMultipleMutationVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: {
      message: string;
      args: MessageCreateMultipleMutationVariables;
    };
  }
>("graphql/createMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);
  const result = await client.mutation<
    MessageCreateMultipleMutation,
    MessageCreateMultipleMutationVariables
  >(MessageCreateMultipleDocument, args);

  if (result.error) {
    thunkAPI.dispatch(setError(result.error.message));
    return thunkAPI.rejectWithValue({
      message: result.error.message,
      args,
    });
  } else if (!result.data) {
    return thunkAPI.rejectWithValue({
      message: "create message: no data in response",
      args,
    });
  }
  // TODO: add the message to the message list
  return thunkAPI.fulfillWithValue(result.data);
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
  const action = createMessage({
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

export const createThreadWithMessage = createAsyncThunk<
  MessageCreateMultipleMutation & CreateThreadMutation,
  {
    content: string;
    expertId: string;
    model: string;
    tools: (Tool["spec"] | FCloudTool)[];
  },
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("flexus/createThreadWithMessage", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const port = state.config.lspPort;
  // TODO: where is current workspace set?
  const workspace =
    state.teams.workspace?.ws_root_group_id ??
    state.config.currentWorkspaceName ??
    "";

  const appIdQuery = await fetchAppSearchableId(apiKey, port);

  if (appIdQuery.error) {
    thunkAPI.rejectWithValue({ message: JSON.stringify(appIdQuery.error) });
  }

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);

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
    return thunkAPI.rejectWithValue({ message: threadQuery.error.message });
  }

  if (!threadQuery.data) {
    return thunkAPI.rejectWithValue({
      message: "couldn't create flexus thread id",
    });
  }

  if (state.threadMessages.ft_id === null) {
    thunkAPI.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
  }

  // Note: ftm_num, ftm_alt, and ftm_prev_alt are also hard coded for tracking waiting state
  const createMessageArgs: FThreadMessageInput = {
    ftm_app_specific: JSON.stringify(appIdQuery.data?.app_searchable_id ?? ""),
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
    return thunkAPI.rejectWithValue({ message: result.error.message });
  }
  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "failed to create message" });
  }
  return thunkAPI.fulfillWithValue({ ...result.data, ...threadQuery.data });
});

export const createThreadWitMultipleMessages = createAsyncThunk<
  MessageCreateMultipleMutation & CreateThreadMutation,
  {
    messages: { ftm_role: string; ftm_content: unknown }[];
    expertId: string;
    model: string;
    tools: (Tool["spec"] | FCloudTool)[];
    integration?: IntegrationMeta;
  },
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string };
  }
>("flexus/createThreadWithMultipleMessages", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";
  const port = state.config.lspPort;
  // TODO: where is current workspace set?
  const workspace =
    state.teams.workspace?.ws_root_group_id ??
    state.config.currentWorkspaceName ??
    "";
  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;
  const appIdQuery = await fetchAppSearchableId(apiKey, port);

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);

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
    return thunkAPI.rejectWithValue({ message: threadQuery.error.message });
  }

  if (!threadQuery.data) {
    return thunkAPI.rejectWithValue({
      message: "couldn't create flexus thread id",
    });
  }

  if (state.threadMessages.ft_id === null) {
    thunkAPI.dispatch(setThreadFtId(threadQuery.data.thread_create.ft_id));
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
    return thunkAPI.rejectWithValue({ message: result.error.message });
  }
  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "failed to create message" });
  }
  return thunkAPI.fulfillWithValue({ ...result.data, ...threadQuery.data });
});

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

export const getExpertsThunk = createAsyncThunk<
  ExpertsForGroupQuery,
  ExpertsForGroupQueryVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; args: ExpertsForGroupQueryVariables };
  }
>("flexus/getExperts", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);

  const result = await client.query<
    ExpertsForGroupQuery,
    ExpertsForGroupQueryVariables
  >(ExpertsForGroupDocument, args);

  if (result.error) {
    return thunkAPI.rejectWithValue({ message: result.error.message, args });
  }
  if (!result.data) {
    return thunkAPI.rejectWithValue({
      message: "failed to get expert data",
      args,
    });
  }

  return thunkAPI.fulfillWithValue(result.data);
});

export const getModelsForExpertThunk = createAsyncThunk<
  ModelsForExpertQuery,
  ModelsForExpertQueryVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; args: ModelsForExpertQueryVariables };
  }
>("flexus/modelsForExpert", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const addressUrl = state.config.addressURL ?? `https://app.refact.ai`;

  const client = createGraphqlClient(addressUrl, apiKey, thunkAPI.signal);

  const result = await client.query<
    ModelsForExpertQuery,
    ModelsForExpertQueryVariables
  >(ModelsForExpertDocument, args);

  if (result.error) {
    return thunkAPI.rejectWithValue({ message: result.error.message, args });
  }

  if (!result.data) {
    return thunkAPI.rejectWithValue({
      message: "error get models for expert",
      args,
    });
  }

  return thunkAPI.fulfillWithValue(result.data);
});

// Note: these could be moved into the slice https://redux-toolkit.js.org/api/createslice#createasyncthunk
export const getToolsForGroupThunk = createAsyncThunk<
  ToolsForGroupQuery,
  ToolsForGroupQueryVariables,
  {
    dispatch: AppDispatch;
    state: RootState;
    rejectValue: { message: string; args: ToolsForGroupQueryVariables };
  }
>("flexus/tools", async (args, thunkAPI) => {
  const state = thunkAPI.getState();
  const apiKey = state.config.apiKey ?? "";

  const client = createGraphqlClient(apiKey, thunkAPI.signal);

  const result = await client.query<
    ToolsForGroupQuery,
    ToolsForGroupQueryVariables
  >(ToolsForGroupDocument, args);

  if (result.error) {
    return thunkAPI.rejectWithValue({ message: result.error.message, args });
  }
  if (!result.data) {
    return thunkAPI.rejectWithValue({ message: "erro fetching tools", args });
  }

  return thunkAPI.fulfillWithValue(result.data);
});

// TODO: patch thread tools

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
