import { createAction, createAsyncThunk } from "@reduxjs/toolkit";
import {
  checkForDetailMessage,
  type ChatThread,
  type PayloadWithId,
  type ToolUse,
} from "./types";
import type {
  ChatMessages,
  ChatResponse,
} from "../../../services/refact/types";
import type { AppDispatch, RootState } from "../../../app/store";
import type { SystemPrompts } from "../../../services/refact/prompts";
import { parseOrElse } from "../../../utils/parseOrElse";
import { formatMessagesForLsp } from "./utils";
import { sendChat } from "../../../services/refact/chat";
import { ToolCommand } from "../../../services/refact/tools";

export const newChatAction = createAction("chatThread/new");

export const chatResponse = createAction<PayloadWithId & ChatResponse>(
  "chatThread/response",
);

export const chatAskedQuestion = createAction<PayloadWithId>(
  "chatThread/askQuestion",
);

export const backUpMessages = createAction<
  PayloadWithId & { messages: ChatThread["messages"] }
>("chatThread/backUpMessages");

// TODO: add history actions to this, maybe not used any more
export const chatError = createAction<PayloadWithId & { message: string }>(
  "chatThread/error",
);

// TODO: include history actions with this one, this could be done by making it a thunk, or use reduce-reducers.
export const doneStreaming = createAction<PayloadWithId>(
  "chatThread/doneStreaming",
);

export const setChatModel = createAction<string>("chatThread/setChatModel");
export const getSelectedChatModel = (state: RootState) =>
  state.chat.thread.model;

export const setSystemPrompt = createAction<SystemPrompts>(
  "chatThread/setSystemPrompt",
);

export const removeChatFromCache = createAction<PayloadWithId>(
  "chatThread/removeChatFromCache",
);

export const restoreChat = createAction<ChatThread>("chatThread/restoreChat");

export const clearChatError = createAction<PayloadWithId>(
  "chatThread/clearError",
);

export const enableSend = createAction<PayloadWithId>("chatThread/enableSend");
export const setPreventSend = createAction<PayloadWithId>(
  "chatThread/preventSend",
);

export const setToolUse = createAction<ToolUse>("chatThread/setToolUse");

// TODO: This is the circular dep when imported from hooks :/
const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

function isValidBuffer(buffer: Uint8Array): boolean {
  // Check if the buffer is long enough
  if (buffer.length < 8) return false; // "data: " is 6 bytes + 2 bytes for "\n\n"

  // Check the start for "data: "
  const startsWithData =
    buffer[0] === 100 && // 'd'
    buffer[1] === 97 && // 'a'
    buffer[2] === 116 && // 't'
    buffer[3] === 97 && // 'a'
    buffer[4] === 58 && // ':'
    buffer[5] === 32; // ' '

  // Check the end for "\n\n"
  const endsWithNewline =
    buffer[buffer.length - 2] === 10 && // '\n'
    buffer[buffer.length - 1] === 10; // '\n'

  // could be detail message
  return startsWithData && endsWithNewline;
}

export const chatAskQuestionThunk = createAppAsyncThunk<
  unknown,
  {
    messages: ChatMessages;
    chatId: string;
    tools: ToolCommand[] | null;
  }
>("chatThread/sendChat", ({ messages, chatId, tools }, thunkAPI) => {
  const state = thunkAPI.getState();

  const messagesForLsp = formatMessagesForLsp(messages);
  return sendChat({
    messages: messagesForLsp,
    model: state.chat.thread.model,
    tools,
    stream: true,
    abortSignal: thunkAPI.signal,
    chatId,
    apiKey: state.config.apiKey,
    port: state.config.lspPort,
  })
    .then((response) => {
      if (!response.ok) {
        return Promise.reject(new Error(response.statusText));
      }

      const decoder = new TextDecoder();
      const reader = response.body?.getReader();
      if (!reader) return;

      return reader.read().then(function pump({ done, value }): Promise<void> {
        if (done) return Promise.resolve();
        if (thunkAPI.signal.aborted) {
          thunkAPI.dispatch(setPreventSend({ id: chatId }));
          return Promise.resolve();
        }

        // TODO: handle details
        if (!isValidBuffer(value)) {
          return reader.read().then(({ done, value: v }) => {
            const buff = new Uint8Array(value.length + (v?.length ?? 0));
            buff.set(value);
            if (v) {
              buff.set(v, value.length);
            }
            return pump({ done, value: buff });
          });
        }
        // accumulate data in binary, buffer
        // packet divided, incomplet unicode
        const streamAsString = decoder.decode(value);
        // if string doesn't end with \n\n then it's not complete, maybe memotising it for the next call could work?
        // if (streamAsString.endsWith("\n\n") === false) {
        //   console.error("Stream was chunked badly");
        // }

        const maybeError = checkForDetailMessage(streamAsString);
        if (maybeError) {
          const error = new Error(maybeError.detail);
          throw error;
        }

        const deltas = streamAsString
          .split("\n\n")
          .filter((str) => str.length > 0);

        if (deltas.length === 0) return Promise.resolve();

        // could be improved
        for (const delta of deltas) {
          // can have error here.
          // incomplete delta ?
          if (!delta.startsWith("data: ")) {
            // eslint-disable-next-line no-console
            console.log("Unexpected data in streaming buf: " + delta);
            continue;
          }

          const maybeJsonString = delta.substring(6);

          if (maybeJsonString === "[DONE]") return Promise.resolve();

          if (maybeJsonString === "[ERROR]") {
            // check for error details
            const errorMessage = "error from lsp";
            const error = new Error(errorMessage);

            return Promise.reject(error);
          }

          const maybeErrorData = checkForDetailMessage(maybeJsonString);
          if (maybeErrorData) {
            const errorMessage: string =
              typeof maybeErrorData.detail === "string"
                ? maybeErrorData.detail
                : JSON.stringify(maybeErrorData.detail);
            const error = new Error(errorMessage);
            // eslint-disable-next-line no-console
            console.error(error);
            throw error;
          }
          const json = parseOrElse<Record<string, unknown>>(
            maybeJsonString,
            {},
          );

          // TODO: type check this. also some models create a new id :/
          thunkAPI.dispatch(
            chatResponse({ ...(json as ChatResponse), id: chatId }),
          );
        }

        return reader.read().then(pump);
      });
    })
    .catch((err: Error) => {
      // console.log("Catch called");
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
      thunkAPI.dispatch(chatError({ id: chatId, message: err.message }));
      return thunkAPI.rejectWithValue(err.message);
    })
    .finally(() => {
      thunkAPI.dispatch(doneStreaming({ id: chatId }));
    });
});
