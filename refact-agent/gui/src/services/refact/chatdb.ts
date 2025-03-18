import { createAsyncThunk } from "@reduxjs/toolkit/react";
import { AppDispatch, RootState } from "../../app/store";
import {
  CHAT_DB_THREADS_SUB,
  CHAT_DB_MESSAGES_SUB,
  CHAT_DB_MESSAGES_UPDATE,
  CHAT_DB_THREADS_UPDATE,
} from "./consts";
import { consumeStream } from "../../features/Chat/Thread/utils";
import {
  isCThreadSubResponseUpdate,
  isCThreadSubResponseDelete,
  isCMessageUpdateResponse,
  CMessage,
  CThread,
  CMessageFromChatDB,
} from "./types";
import { chatDbActions } from "../../features/ChatDB/chatDbSlice";
import { chatDbMessageSliceActions } from "../../features/ChatDB/chatDbMessagesSlice";

const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

export type SubscribeToThreadArgs =
  | {
      quick_search?: string;
      limit?: number;
    }
  | undefined;
function subscribeToThreads(
  args: SubscribeToThreadArgs = {},
  port = 8001,
  apiKey?: string | null,
  abortSignal?: AbortSignal,
): Promise<Response> {
  const url = `http://127.0.0.1:${port}${CHAT_DB_THREADS_SUB}`;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");
  if (apiKey) {
    headers.append("Authorization", `Bearer ${apiKey}`);
  }

  return fetch(url, {
    method: "POST",
    headers,
    redirect: "follow",
    cache: "no-cache",
    body: JSON.stringify(args),
    signal: abortSignal,
  });
}

// type CThreadSubResponse = CThreadSubResponseUpdate | CThreadSubResponseDelete;
// function isCThreadSubResponseChunk(value: unknown): value is CThreadSubResponse {
//   if (isCThreadSubResponseUpdate(value)) return true;
//   if (isCThreadSubResponseDelete(value)) return true;
//   return false;
// }

export const subscribeToThreadsThunk = createAppAsyncThunk<
  unknown,
  SubscribeToThreadArgs
>("chatdbApi/subscribeToThreads", (args, thunkApi) => {
  const state = thunkApi.getState() as unknown as RootState;
  const port = state.config.lspPort;
  const apiKey = state.config.apiKey;
  return subscribeToThreads(args, port, apiKey, thunkApi.signal)
    .then((response) => {
      if (!response.ok) {
        throw new Error(response.statusText);
      }
      const reader = response.body?.getReader();
      if (!reader) return;

      const onAbort = () => {
        console.log("knowledge stream aborted");
      };

      const onChunk = (chunk: unknown) => {
        if (isCThreadSubResponseUpdate(chunk)) {
          const action = chatDbActions.updateCThread(chunk.cthread_rec);
          thunkApi.dispatch(action);
          // dispatch update
        } else if (isCThreadSubResponseDelete(chunk)) {
          const action = chatDbActions.deleteCThread(chunk.cthread_id);
          thunkApi.dispatch(action);
          // dispatch delete
        } else {
          console.log("unknown thread chunk", chunk);
        }
      };

      return consumeStream(reader, thunkApi.signal, onAbort, onChunk);
    })
    .catch((err) => {
      // eslint-disable-next-line no-console
      console.error("Error in chat thread subscription", err);
      // todo: handle error
    });
});

export function updateThread(
  thread: Partial<CThread> & Pick<CThread, "cthread_id">,
  port = 8001,
  apiKey?: string | null,
): Promise<Response> {
  const url = `http://127.0.0.1:${port}${CHAT_DB_THREADS_UPDATE}`;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");
  if (apiKey) {
    headers.append("Authorization", `Bearer ${apiKey}`);
  }

  return fetch(url, {
    method: "POST",
    headers,
    redirect: "follow",
    cache: "no-cache",
    body: JSON.stringify(thread),
  });
}

export const updateThreadThunk = createAppAsyncThunk<
  unknown,
  Partial<CThread> & Pick<CThread, "cthread_id">
>("chatDbApi/updateThread", (thread, thunkApi) => {
  const state = thunkApi.getState() as unknown as RootState;
  const port = state.config.lspPort;
  const apiKey = state.config.apiKey;
  const updatedThread = {
    ...thread,
    cthread_updated_ts: Date.now(),
    cthread_created_ts: thread.cthread_created_ts ?? Date.now(),
  };
  return updateThread(updatedThread, port, apiKey).then((res) => {
    return res.json();
  });
});

function subscribeToThreadMessages(
  cthreadId: string,
  port = 8001,
  apiKey?: string | null,
  abortSignal?: AbortSignal,
): Promise<Response> {
  const url = `http://127.0.0.1:${port}${CHAT_DB_MESSAGES_SUB}`;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");
  if (apiKey) {
    headers.append("Authorization", `Bearer ${apiKey}`);
  }

  return fetch(url, {
    method: "POST",
    headers,
    redirect: "follow",
    cache: "no-cache",
    body: JSON.stringify({ cmessage_belongs_to_cthread_id: cthreadId }),
    signal: abortSignal,
  });
}

export const subscribeToThreadMessagesThunk = createAppAsyncThunk<
  unknown,
  string
>("chatDbApi/subscribeToThreadMessages", (cthreadId, thunkApi) => {
  if (!cthreadId) return;
  const state = thunkApi.getState() as unknown as RootState;
  const port = state.config.lspPort;
  const apiKey = state.config.apiKey;

  return subscribeToThreadMessages(cthreadId, port, apiKey, thunkApi.signal)
    .then((response) => {
      if (!response.ok) {
        throw new Error(response.statusText);
      }
      const reader = response.body?.getReader();
      if (!reader) return;

      const onAbort = () => {
        console.log("message stream aborted");
      };

      const onChunk = (chunk: Record<string, unknown>) => {
        console.log("cmessages chunks");
        console.log({ chunk });
        if (isCMessageUpdateResponse(chunk)) {
          const action = chatDbMessageSliceActions.updateMessage({
            threadId: cthreadId,
            message: chunk.cmessage_rec,
          });
          thunkApi.dispatch(action);
        } else {
          console.log("invalid chunk");
        }
      };

      return consumeStream(reader, thunkApi.signal, onAbort, onChunk);
    })
    .catch((error) => {
      // eslint-disable-next-line no-console
      console.error("Error in chat thread subscription", error);
      // todo: handle error
    });
});

// How does this add a context file ? or images ?
export function updateCMessage(
  cmessages: CMessage[],
  port = 8001,
  apiKey?: string | null,
) {
  const url = `http://127.0.0.1:${port}${CHAT_DB_MESSAGES_UPDATE}`;
  const headers = new Headers();
  headers.append("Content-Type", "application/json");
  if (apiKey) {
    headers.append("Authorization", `Bearer ${apiKey}`);
  }

  const messages: CMessageFromChatDB[] = cmessages.map((message) => {
    return { ...message, cmessage_json: JSON.stringify(message.cmessage_json) };
  });

  return fetch(url, {
    method: "POST",
    headers,
    redirect: "follow",
    cache: "no-cache",
    body: JSON.stringify(messages),
  });
}

export const updateCMessagesThunk = createAppAsyncThunk<unknown, CMessage[]>(
  "chatDbApi/updateCMessagesThunk",
  (cmessages, thunkApi) => {
    const state = thunkApi.getState() as unknown as RootState;
    const port = state.config.lspPort;
    const apiKey = state.config.apiKey;
    return updateCMessage(cmessages, port, apiKey).then((res) => res.json());
  },
);

// export const sendMessagesThunk = createAppAsyncThunk<
//   unknown,
//   {
//     messages: CMessage[];
//   }
// >("chatDbApi/sendThreadAndMessagesThunk", async (args, thunkApi) => {
//   if (args.messages.length === 0) return;
//   const id = args.messages.map((m) => m.cmessage_belongs_to_cthread_id)[0];
//   await thunkApi.dispatch(
//     updateThreadThunk({
//       cthread_id: id,
//       cthread_updated_ts: new Date().getTime(),
//     }),
//   );

//   await thunkApi.dispatch(updateCMessagesThunk(args.messages));
// });

// TODO: create thread an subscribe to messages

// Types for the API

// export interface Chore {
//   chore_id: string;
//   chore_title: string;
//   chore_spontaneous_work_enable: boolean;
//   chore_created_ts: number;
//   chore_archived_ts: number;
// }

// export interface ChoreEvent {
//   chore_event_id: string;
//   chore_event_belongs_to_chore_id: string;
//   chore_event_summary: string;
//   chore_event_ts: number;
//   chore_event_link: string;
//   chore_event_cthread_id: string | null;
// }

// // Request types
// export interface CThreadSubscription {
//   quicksearch?: string;
//   limit?: number;
// }

// export interface CMessagesSubscription {
//   cmessage_belongs_to_cthread_id: string;
// }

// API definition
// export const chatDbApi = createApi({
//   reducerPath: "chatdbApi",
//   baseQuery: fetchBaseQuery({
//     prepareHeaders: (headers, { getState }) => {
//       const token = (getState() as RootState).config.apiKey;
//       if (token) {
//         headers.set("Authorization", `Bearer ${token}`);
//       }
//       return headers;
//     },
//   }),
//   endpoints: (builder) => ({
//     // Threads
//     subscribeCThreads: builder.mutation<void, CThreadSubscription>({
//       query: (subscription) => ({
//         url: "/cthreads-sub",
//         method: "POST",
//         body: subscription,
//       }),
//     }),
//     updateCThread: builder.mutation<
//       { status: string; cthread: CThread },
//       Partial<CThread>
//     >({
//       query: (thread) => ({
//         url: "/cthread-update",
//         method: "POST",
//         body: thread,
//       }),
//     }),

//     // Messages
//     subscribeCMessages: builder.mutation<void, CMessagesSubscription>({
//       query: (subscription) => ({
//         url: "/cmessages-sub",
//         method: "POST",
//         body: subscription,
//       }),
//     }),
//     updateCMessages: builder.mutation<{ status: string }, CMessage[]>({
//       query: (messages) => ({
//         url: "/cmessages-update",
//         method: "POST",
//         body: messages,
//       }),
//     }),

//     // Chores
//     subscribeChores: builder.mutation<void, void>({
//       query: () => ({
//         url: "/chores-sub",
//         method: "POST",
//       }),
//     }),
//     updateChore: builder.mutation<{ status: string }, Partial<Chore>>({
//       query: (chore) => ({
//         url: "/chore-update",
//         method: "POST",
//         body: chore,
//       }),
//     }),
//     updateChoreEvent: builder.mutation<{ status: string }, Partial<ChoreEvent>>(
//       {
//         query: (event) => ({
//           url: "/chore-event-update",
//           method: "POST",
//           body: event,
//         }),
//       },
//     ),
//   }),
// });

// // Export hooks for usage in components
// export const {
//   useSubscribeCThreadsMutation,
//   useUpdateCThreadMutation,
//   useSubscribeCMessagesMutation,
//   useUpdateCMessagesMutation,
//   useSubscribeChoresMutation,
//   useUpdateChoreMutation,
//   useUpdateChoreEventMutation,
// } = chatDbApi;
