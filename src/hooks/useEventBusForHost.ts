import { useEffect, useRef, useState } from "react";
import {
  sendChat,
  getCaps,
  getAtCommandCompletion,
  getAtCommandPreview,
  isDetailMessage,
  getPrompts,
  formatMessagesForLsp,
  getAvailableTools,
  ToolCommand,
  // LspChatMessage,
  checkDiff,
} from "../services/refact";
import { useChatHistory } from "./useChatHistory";
import {
  EVENT_NAMES_TO_CHAT,
  EVENT_NAMES_TO_STATISTIC,
  ChatThread,
  isQuestionFromChat,
  isSaveChatFromChat,
  isRequestCapsFromChat,
  isStopStreamingFromChat,
  isRequestDataForStatistic,
  isRequestAtCommandCompletion,
  ReceiveAtCommandCompletion,
  ReceiveAtCommandPreview,
  isRequestPrompts,
  ReceivePrompts,
  ReceivePromptsError,
  isRequestPreviewFiles,
  isTakeNotesFromChat,
  isRequestTools,
  RecieveTools,
  isReadyMessage,
  isRequestDiffAppliedChunks,
  RecieveDiffAppliedChunks,
  RecieveDiffAppliedChunksError,
} from "../events";
import { useConfig } from "../contexts/config-context";
import { getStatisticData } from "../services/refact";
import { parseOrElse } from "../utils";

export function useEventBusForHost() {
  const { lspUrl } = useConfig();
  const { saveChat } = useChatHistory();
  const [currentChatId, setCuttentChat] = useState("");
  // this needs to be a ref because it is mutated in a useEffect
  const controller = useRef(new AbortController());

  const [takeingNotes] = useState<boolean>(false);
  // const [takeingNotes, setTakingNotes] = useState<boolean>(false);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }

      if (isReadyMessage(event.data)) {
        setCuttentChat(event.data.payload.id);
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

        handleSend(
          event.data.payload,
          controller.current,
          event.data.payload.tools,
          lspUrl,
        );
        return;
      }

      if (isTakeNotesFromChat(event.data)) {
        // TAKE_NOTES
        // disable memory for the stable release
        // setTakingNotes(true);
        // const chatId: string = event.data.payload.id;
        // const { messages, model } = event.data.payload;
        // // console.log({ messages });
        // const messagesForLsp = formatMessagesForLsp(messages);
        // sendChat({
        //   messages: messagesForLsp,
        //   model,
        //   stream: false,
        //   lspUrl,
        //   takeNote: true,
        // })
        //   .then(async (res) => {
        //     const json = (await res.json()) as {
        //       choices: { message: LspChatMessage }[];
        //       deterministic_messages: LspChatMessage[];
        //       model: string;
        //     };
        //     const choices = json.choices.map((choice) => choice.message);
        //     const messagesAndNotes = [
        //       ...messagesForLsp.slice(0, -1), // remove duplicated user message
        //       ...json.deterministic_messages,
        //       ...choices,
        //     ];
        //     return sendChat({
        //       messages: messagesAndNotes,
        //       model: model,
        //       stream: false,
        //       lspUrl,
        //       takeNote: true,
        //       onlyDeterministicMessages: true,
        //       chatId: chatId,
        //     });
        //   })
        //   .then((res) => res.json())
        //   .catch((err) => {
        //     // eslint-disable-next-line no-console
        //     console.error(err);
        //   })
        //   .finally(() => setTakingNotes(false));
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

      if (isRequestAtCommandCompletion(event.data)) {
        const { id, query, cursor, number } = event.data.payload;

        getAtCommandCompletion(query, cursor, number, lspUrl)
          .then((res) => {
            if (isDetailMessage(res)) return;
            const message: ReceiveAtCommandCompletion = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_COMPLETION,
              payload: { ...res, id },
            };

            window.postMessage(message, "*");
          })
          .catch((error) => {
            // eslint-disable-next-line no-console
            console.error(error);
          });
      }

      if (isRequestPreviewFiles(event.data)) {
        const { query, id } = event.data.payload;

        getAtCommandPreview(query, lspUrl)
          .then((res) => {
            if (isDetailMessage(res)) return;
            const message: ReceiveAtCommandPreview = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_PREVIEW,
              payload: { id, preview: res },
            };
            window.postMessage(message, "*");
          })
          .catch((error) => {
            // eslint-disable-next-line no-console
            console.error(error);
          });
      }

      if (isRequestDataForStatistic(event.data)) {
        getStatisticData(lspUrl)
          .then((data) => {
            window.postMessage(
              {
                type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA,
                payload: data,
              },
              "*",
            );
          })
          .catch((error: Error) => {
            window.postMessage(
              {
                type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR,
                payload: {
                  message: error.message,
                },
              },
              "*",
            );
          });
      }

      if (isRequestPrompts(event.data)) {
        const id = event.data.payload.id;
        getPrompts(lspUrl)
          .then((prompts) => {
            const message: ReceivePrompts = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS,
              payload: { id, prompts },
            };

            window.postMessage(message, "*");
          })
          .catch((error: Error) => {
            const message: ReceivePromptsError = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS_ERROR,
              payload: { id, error: `Prompts: ${error.message}` },
            };

            window.postMessage(message, "*");
          });
      }

      if (isRequestTools(event.data)) {
        const id = event.data.payload.id;
        getAvailableTools(lspUrl)
          .then((tools) => {
            const action: RecieveTools = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_TOOLS,
              payload: { id, tools },
            };
            window.postMessage(action, "*");
          })
          .catch(() => {
            const action: RecieveTools = {
              type: EVENT_NAMES_TO_CHAT.RECEIVE_TOOLS,
              payload: { id, tools: [] },
            };

            window.postMessage(action, "*");
          });
      }

      if (isRequestDiffAppliedChunks(event.data)) {
        const { id, message_id } = event.data.payload;
        checkDiff({
          chat_id: id,
          message_id,
        })
          .then((res) => {
            const action: RecieveDiffAppliedChunks = {
              type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS,
              payload: {
                id,
                message_id,
                applied_chunks: res.applied_chunks,
              },
            };
            window.postMessage(action, "*");
          })
          .catch((err: Error) => {
            const action: RecieveDiffAppliedChunksError = {
              type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS_ERROR,
              payload: { id, message_id, reason: err.message },
            };
            window.postMessage(action, "*");
          });
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [saveChat, lspUrl]);

  return {
    takeingNotes,
    currentChatId,
  };
}

function handleSend(
  chat: ChatThread,
  controller: AbortController,
  tools: ToolCommand[] | null,
  lspUrl?: string,
) {
  const messages = formatMessagesForLsp(chat.messages);
  sendChat({
    messages,
    model: chat.model,
    abortController: controller,
    stream: true,
    tools,
    lspUrl,
  })
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
            // check for error details
            const errorMessage = "error from lsp";
            const error = new Error(errorMessage);

            return Promise.reject(error); // handle error
          }

          // figure out how to safely parseJson

          const json = parseOrElse<Record<string, unknown>>(
            maybeJsonString,
            {},
          );

          if ("detail" in json) {
            const errorMessage: string =
              typeof json.detail === "string"
                ? json.detail
                : JSON.stringify(json.detail);
            const error = new Error(errorMessage);

            // eslint-disable-next-line no-console
            console.error(error);
            return Promise.reject(error);
          }
          window.postMessage(
            {
              type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
              payload: {
                ...json,
                id: chat.id,
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
