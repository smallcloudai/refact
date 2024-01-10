import { useEffect, useRef } from "react";
import { sendChat, getCaps, ChatContextFile } from "../services/refact";
import { useChatHistory } from "./useChatHistory";
import {
  EVENT_NAMES_TO_CHAT,
  ChatThread,
  isQuestionFromChat,
  isSaveChatFromChat,
  isRequestCapsFromChat,
  isStopStreamingFromChat,
  isRequestForFileFromChat,
} from "../events";
import { useConfig } from "../contexts/config-context";

export function useEventBusForHost() {
  const { lspUrl } = useConfig();
  const { saveChat } = useChatHistory();
  // this needs to be a ref because it is mutated in a useEffect
  const controller = useRef(new AbortController());

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (isStopStreamingFromChat(event.data)) {
        controller.current.abort();
        controller.current = new AbortController();
        return;
      }

      if (isQuestionFromChat(event.data)) {
        const payload = event.data.payload;

        saveChat({
          id: payload.id,
          title: payload.title ?? "",
          messages: payload.messages,
          model: payload.model,
        });

        handleSend(event.data.payload, controller.current, lspUrl);
        return;
      }

      if (isSaveChatFromChat(event.data)) {
        const chat = event.data.payload;
        saveChat(chat);
      }

      if (isRequestCapsFromChat(event.data)) {
        const chat_id = event.data.payload.id;
        getCaps(lspUrl)
          .then((caps) => {
            window.postMessage({
              type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS,
              payload: {
                id: chat_id,
                caps,
              },
            });
          })
          .catch((error: Error) => {
            window.postMessage({
              type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS_ERROR,
              payload: {
                id: chat_id,
                message: error.message,
              },
            });
          });
      }

      if (isRequestForFileFromChat(event.data)) {
        const { payload } = event.data;

        window
          .showOpenFilePicker({ multiple: true })
          .then(async (fileHandlers) => {
            const promises = fileHandlers.map(async (fileHandler) => {
              const file = await fileHandler.getFile();
              const content = await file.text();
              const messageInChat: ChatContextFile = {
                file_name: fileHandler.name,
                file_content: content,
                line1: 1,
                line2: content.split("\n").length + 1,
              };
              return messageInChat;
            });

            const files = await Promise.all(promises);
            window.postMessage({
              type: EVENT_NAMES_TO_CHAT.RECEIVE_FILES,
              payload: {
                id: payload.id,
                files,
              },
            });
          })
          .catch((error: Error) => {
            if (error instanceof DOMException && error.name === "AbortError") {
              return;
            }
            // eslint-disable-next-line no-console
            console.error(error);

            // TODO: add specific error type for this case
            window.postMessage(
              {
                type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING,
                payload: {
                  id: payload.id,
                  message: error.message || "error attaching file",
                },
              },
              "*",
            );
          });
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [saveChat, lspUrl]);
}

function handleSend(
  chat: ChatThread,
  controller: AbortController,
  lspUrl?: string,
) {
  sendChat(chat.messages, chat.model, controller, lspUrl)
    .then((response) => {
      if (!response.ok) {
        return Promise.reject(new Error(response.statusText));
      }
      const decoder = new TextDecoder();
      const reader = response.body?.getReader();
      if (!reader) return;
      return reader.read().then(function pump({ done, value }): Promise<void> {
        if (done) {
          // Do something with last chunk of data then exit reader
          return Promise.resolve();
        }
        if (controller.signal.aborted) {
          return Promise.resolve();
        }

        const streamAsString = decoder.decode(value);

        const deltas = streamAsString
          .split("\n\n")
          .filter((str) => str.length > 0);
        if (deltas.length === 0) return Promise.resolve();

        for (const delta of deltas) {
          if (!delta.startsWith("data: ")) {
            // eslint-disable-next-line no-console
            console.log("Unexpected data in streaming buf: " + delta);
            continue;
          }

          const maybeJsonString = delta.substring(6);
          if (maybeJsonString === "[DONE]") {
            window.postMessage(
              {
                type: EVENT_NAMES_TO_CHAT.DONE_STREAMING,
                payload: { id: chat.id },
              },
              "*",
            );
            return Promise.resolve(); // handle finish
          }

          if (maybeJsonString === "[ERROR]") {
            // TODO safely parse json
            const errorJson = JSON.parse(maybeJsonString) as Record<
              string,
              unknown
            >;
            const errorMessage =
              typeof errorJson.detail === "string"
                ? errorJson.detail
                : "error from lsp";
            const error = new Error(errorMessage);

            return Promise.reject(error); // handle error
          }
          // figure out how to safely parseJson

          const json = JSON.parse(maybeJsonString) as Record<string, unknown>;

          if ("detail" in json) {
            const errorMessage: string =
              typeof json.detail === "string" ? json.detail : "error from lsp";
            const error = new Error(errorMessage);
            return Promise.reject(error);
          }
          window.postMessage(
            {
              type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
              payload: {
                id: chat.id,
                ...json,
              },
            },
            "*",
          );
        }

        return reader.read().then(pump);
      });
    })
    .catch((error: Error) => {
      if (!controller.signal.aborted) {
        // eslint-disable-next-line no-console
        console.error(error);
        window.postMessage(
          {
            type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING,
            payload: {
              id: chat.id,
              message: error.message,
            },
          },
          "*",
        );
      }
    })
    .finally(() => {
      window.postMessage(
        { type: EVENT_NAMES_TO_CHAT.DONE_STREAMING, payload: { id: chat.id } },
        "*",
      );
    });
}
