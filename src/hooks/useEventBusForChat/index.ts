import { useEffect, useReducer, useCallback, useMemo } from "react";
import {
  type ChatContextFile,
  type ChatMessages,
  type ChatResponse,
  isChatContextFileMessage,
} from "../../services/refact";
import { v4 as uuidv4 } from "uuid";
import {
  EVENT_NAMES_TO_CHAT,
  EVENT_NAMES_FROM_CHAT,
  isActionToChat,
  type ActionToChat,
  type ChatThread,
  isResponseToChat,
  isBackupMessages,
  isRestoreChat,
  isChatDoneStreaming,
  isChatErrorStreaming,
  isChatClearError,
  isChatReceiveCaps,
  isRequestCapsFromChat,
  isCreateNewChat,
  isChatReceiveCapsError,
  isSetChatModel,
  isSetDisableChat,
  isActiveFileInfo,
  type NewFileFromChat,
  type PasteDiffFromChat,
  type ReadyMessage,
  type RequestAtCommandCompletion,
  isReceiveAtCommandCompletion,
  type SetSelectedAtCommand,
  isSetSelectedAtCommand,
  isReceiveAtCommandPreview,
  isChatUserMessageResponse,
  isChatSetLastModelUsed,
  isSetSelectedSnippet,
  isRemovePreviewFileByName,
  type RemovePreviewFileByName,
  isSetPreviousMessagesLength,
  setPreviousMessagesLength,
  type Snippet,
  isReceiveTokenCount,
  type FileInfo,
  type ChatSetSelectedSnippet,
  type CreateNewChatThread,
  type SaveChatFromChat,
  isReceivePrompts,
  isRequestPrompts,
  isReceivePromptsError,
  type RequestPrompts,
  isSetSelectedSystemPrompt,
  type SetSelectedSystemPrompt,
  type SystemPrompts,
} from "../../events";
import { usePostMessage } from "../usePostMessage";
import { useDebounceCallback } from "usehooks-ts";

function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  if (isChatUserMessageResponse(response)) {
    if (response.role === "context_file") {
      return [...messages, [response.role, JSON.parse(response.content)]];
    }
    return [...messages, [response.role, response.content]];
  }

  return response.choices.reduce<ChatMessages>((acc, cur) => {
    if (cur.delta.role === "context_file") {
      return acc.concat([[cur.delta.role, cur.delta.content]]);
    }

    if (acc.length === 0) {
      return acc.concat([[cur.delta.role, cur.delta.content]]);
    }

    const lastMessage = acc[acc.length - 1];
    if (lastMessage[0] === "assistant") {
      const last = acc.slice(0, -1);
      const currentMessage = lastMessage[1];
      return last.concat([
        [cur.delta.role, currentMessage + cur.delta.content],
      ]);
    }

    return acc.concat([[cur.delta.role, cur.delta.content]]);
  }, messages);
}

export function reducer(postMessage: typeof window.postMessage) {
  return function (state: ChatState, action: ActionToChat): ChatState {
    const isThisChat =
      action.payload?.id && action.payload.id === state.chat.id ? true : false;

    // console.log(action.type, { isThisChat });
    // console.log(action.payload);

    if (isThisChat && isSetDisableChat(action)) {
      return {
        ...state,
        streaming: action.payload.disable,
        waiting_for_response: action.payload.disable,
      };
    }

    if (isThisChat && isResponseToChat(action)) {
      const hasUserMessage = isChatUserMessageResponse(action.payload);
      const current = hasUserMessage
        ? state.chat.messages.slice(0, state.previous_message_length)
        : state.chat.messages;
      const messages = formatChatResponse(current, action.payload);
      return {
        ...state,
        waiting_for_response: false,
        streaming: true,
        previous_message_length: messages.length,
        files_in_preview: [],
        chat: {
          ...state.chat,
          messages,
        },
      };
    }

    if (isThisChat && isBackupMessages(action)) {
      return {
        ...state,
        error: null,
        chat: {
          ...state.chat,
          messages: action.payload.messages,
        },
      };
    }

    if (isThisChat && isRestoreChat(action)) {
      const messages: ChatMessages = action.payload.chat.messages.map(
        (message) => {
          if (message[0] === "context_file" && typeof message[1] === "string") {
            let file: ChatContextFile[] = [];
            try {
              file = JSON.parse(message[1]) as ChatContextFile[];
            } catch {
              file = [];
            }
            return [message[0], file];
          }

          return message;
        },
      );

      return {
        ...state,
        waiting_for_response: false,
        streaming: false,
        error: null,
        previous_message_length: messages.length,
        chat: {
          ...action.payload.chat,
          messages,
        },
        selected_snippet: action.payload.snippet ?? state.selected_snippet,
      };
    }

    if (isThisChat && isCreateNewChat(action)) {
      const nextState = createInitialState();

      return {
        ...nextState,
        chat: {
          ...nextState.chat,
          model: state.chat.model,
        },
        selected_snippet: action.payload?.snippet ?? state.selected_snippet,
      };
    }

    if (isRequestCapsFromChat(action)) {
      return {
        ...state,
        caps: {
          ...state.caps,
          fetching: true,
        },
      };
    }

    if (isThisChat && isChatReceiveCaps(action)) {
      const default_cap = action.payload.caps.code_chat_default_model;
      const available_caps = Object.keys(action.payload.caps.code_chat_models);
      const error = available_caps.length === 0 ? "No available caps" : null;

      return {
        ...state,
        error,
        caps: {
          fetching: false,
          default_cap: default_cap || available_caps[0] || "",
          available_caps,
          error: null,
        },
      };
    }

    if (isThisChat && isChatReceiveCapsError(action)) {
      return {
        ...state,
        error: state.caps.error ? null : action.payload.message,
        caps: {
          ...state.caps,
          fetching: false,
          error: action.payload.message,
        },
      };
    }

    if (isThisChat && isChatDoneStreaming(action)) {
      postMessage({
        type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT,
        payload: state.chat,
      });

      return {
        ...state,
        waiting_for_response: false,
        streaming: false,
      };
    }

    if (isThisChat && isChatErrorStreaming(action)) {
      return {
        ...state,
        streaming: false,
        waiting_for_response: false,
        error:
          typeof action.payload.message === "string"
            ? action.payload.message
            : "Error streaming",
      };
    }

    if (isThisChat && isChatClearError(action)) {
      return {
        ...state,
        error: null,
      };
    }

    if (isThisChat && isSetChatModel(action)) {
      return {
        ...state,
        chat: {
          ...state.chat,
          model: action.payload.model,
        },
      };
    }

    if (isThisChat && isActiveFileInfo(action)) {
      return {
        ...state,
        active_file: {
          ...state.active_file,
          ...action.payload.file,
        },
      };
    }

    if (isThisChat && isReceiveAtCommandCompletion(action)) {
      const selectedCommand = state.rag_commands.selected_command;
      const availableCommands = selectedCommand
        ? state.rag_commands.available_commands
        : action.payload.completions;
      const args = selectedCommand ? action.payload.completions : [];
      return {
        ...state,
        rag_commands: {
          ...state.rag_commands,
          available_commands: availableCommands,
          arguments: args,
          is_cmd_executable: action.payload.is_cmd_executable,
        },
      };
    }

    if (isThisChat && isSetSelectedAtCommand(action)) {
      return {
        ...state,
        rag_commands: {
          ...state.rag_commands,
          selected_command: action.payload.command,
        },
      };
    }

    if (isThisChat && isReceiveAtCommandPreview(action)) {
      const filesInPreview = action.payload.preview.reduce<ChatContextFile[]>(
        (acc, curr) => {
          const files = curr[1];
          return [...acc, ...files];
        },
        [],
      );

      return {
        ...state,
        files_in_preview: filesInPreview,
      };
    }

    // TODO: this may need to be set by the editor
    if (isThisChat && isChatSetLastModelUsed(action)) {
      return {
        ...state,
        chat: {
          ...state.chat,
          model: action.payload.model,
        },
      };
    }

    if (isThisChat && isSetSelectedSnippet(action)) {
      return {
        ...state,
        selected_snippet: action.payload.snippet,
      };
    }

    if (isThisChat && isRemovePreviewFileByName(action)) {
      const previewFiles = state.files_in_preview.filter(
        (file) => file.file_name !== action.payload.name,
      );
      return {
        ...state,
        files_in_preview: previewFiles,
      };
    }

    if (isThisChat && isSetPreviousMessagesLength(action)) {
      return {
        ...state,
        previous_message_length: action.payload.message_length,
      };
    }

    if (isThisChat && isReceiveTokenCount(action)) {
      return {
        ...state,
        tokens: action.payload.tokens,
      };
    }

    if (isThisChat && isRequestPrompts(action)) {
      return {
        ...state,
        system_prompts: {
          ...state.system_prompts,
          fetching: true,
        },
      };
    }

    if (isThisChat && isReceivePrompts(action)) {
      const maybeDefault: string | null =
        "default" in action.payload.prompts
          ? action.payload.prompts.default.text
          : null;
      return {
        ...state,
        selected_system_prompt: state.selected_system_prompt ?? maybeDefault,
        system_prompts: {
          error: null,
          fetching: false,
          prompts: action.payload.prompts,
        },
      };
    }

    if (isThisChat && isReceivePromptsError(action)) {
      return {
        ...state,
        error: state.system_prompts.error ? null : action.payload.error,
        system_prompts: {
          ...state.system_prompts,
          error: action.payload.error,
          fetching: false,
        },
      };
    }

    if (isThisChat && isSetSelectedSystemPrompt(action)) {
      return {
        ...state,
        selected_system_prompt: action.payload.prompt,
      };
    }

    return state;
  };
}

export type ChatCapsState = {
  fetching: boolean;
  default_cap: string;
  available_caps: string[];
  error: null | string;
};

export type ChatState = {
  chat: ChatThread;
  waiting_for_response: boolean;
  streaming: boolean;
  previous_message_length: number;
  error: string | null;
  caps: ChatCapsState;
  rag_commands: {
    available_commands: string[];
    selected_command: string;
    arguments: string[];
    is_cmd_executable: boolean;
  };
  files_in_preview: ChatContextFile[];
  active_file: FileInfo;
  selected_snippet: Snippet;
  tokens: number | null;
  system_prompts: {
    error: null | string;
    prompts: SystemPrompts;
    fetching: boolean;
  };
  selected_system_prompt: null | string;
};

export function createInitialState(): ChatState {
  return {
    streaming: false,
    waiting_for_response: false,
    error: null,
    previous_message_length: 0,
    selected_snippet: {
      language: "",
      code: "",
      path: "",
      basename: "",
    },
    files_in_preview: [],
    chat: {
      id: uuidv4(),
      messages: [],
      title: "",
      model: "",
    },
    caps: {
      fetching: false,
      default_cap: "",
      available_caps: [],
      error: null,
    },
    rag_commands: {
      available_commands: [],
      selected_command: "",
      arguments: [],
      is_cmd_executable: false,
    },

    active_file: {
      name: "",
      line1: null,
      line2: null,
      attach: false,
      can_paste: false,
      path: "",
      cursor: null,
    },
    tokens: null,
    system_prompts: {
      error: null,
      prompts: {},
      fetching: false,
    },
    selected_system_prompt: null,
  };
}

const initialState = createInitialState();
// Maybe use context to avoid prop drilling?
export const useEventBusForChat = () => {
  const postMessage = usePostMessage();
  const [state, dispatch] = useReducer(reducer(postMessage), initialState);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isActionToChat(event.data)) {
        dispatch(event.data);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [state, dispatch]);

  const clearError = useCallback(() => {
    dispatch({
      type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR,
      payload: { id: state.chat.id },
    });
  }, [state.chat.id]);

  const sendMessages = useCallback(
    (messages: ChatMessages, attach_file = state.active_file.attach) => {
      clearError();
      dispatch({
        type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT,
        payload: { id: state.chat.id, disable: true },
      });

      const payload: ChatThread = {
        id: state.chat.id,
        messages: messages,
        title: state.chat.title,
        model: state.chat.model,
        attach_file,
      };

      dispatch({
        type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES,
        payload,
      });
      postMessage({
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload,
      });

      const snippetMessage: ChatSetSelectedSnippet = {
        type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SNIPPET,
        payload: {
          id: state.chat.id,
          snippet: { language: "", code: "", path: "", basename: "" },
        },
      };
      dispatch(snippetMessage);
    },
    [
      clearError,
      postMessage,
      state.active_file.attach,
      state.chat.id,
      state.chat.model,
      state.chat.title,
    ],
  );

  const askQuestion = useCallback(
    (question: string) => {
      const maybeMessagesWithSystemPrompt: ChatMessages =
        state.selected_system_prompt && state.chat.messages.length === 0
          ? [["system", state.selected_system_prompt]]
          : state.chat.messages;
      const messages = maybeMessagesWithSystemPrompt.concat([
        ["user", question],
      ]);
      sendMessages(messages);
    },
    [sendMessages, state.chat.messages, state.selected_system_prompt],
  );

  const requestCaps = useCallback(() => {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS,
      payload: {
        id: state.chat.id,
      },
    });
  }, [postMessage, state.chat.id]);

  const maybeRequestCaps = useCallback(() => {
    if (
      state.chat.messages.length === 0 &&
      state.caps.available_caps.length === 0 &&
      !state.caps.fetching
    ) {
      requestCaps();
    }
  }, [
    state.chat.messages.length,
    state.caps.available_caps.length,
    state.caps.fetching,
    requestCaps,
  ]);

  const requestPrompts = useCallback(() => {
    const message: RequestPrompts = {
      type: EVENT_NAMES_FROM_CHAT.REQUEST_PROMPTS,
      payload: { id: state.chat.id },
    };
    postMessage(message);
  }, [postMessage, state.chat.id]);

  const maybeRequestPrompts = useCallback(() => {
    const hasPrompts = Object.keys(state.system_prompts.prompts).length > 0;
    const hasChat = state.chat.messages.length > 0;
    const isFetching = state.system_prompts.fetching;
    if (!hasPrompts && !hasChat && !isFetching) {
      requestPrompts();
    }
  }, [
    requestPrompts,
    state.chat.messages.length,
    state.system_prompts.fetching,
    state.system_prompts.prompts,
  ]);

  useEffect(() => {
    if (!state.error) {
      maybeRequestCaps();
      maybeRequestPrompts();
    }
  }, [state.error, maybeRequestCaps, maybeRequestPrompts]);

  const setChatModel = useCallback(
    (model: string) => {
      const action = {
        type: EVENT_NAMES_TO_CHAT.SET_CHAT_MODEL,
        payload: {
          id: state.chat.id,
          model: model === state.caps.default_cap ? "" : model,
        },
      };
      dispatch(action);
    },
    [state.chat.id, state.caps.default_cap],
  );

  const stopStreaming = useCallback(() => {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.STOP_STREAMING,
      payload: { id: state.chat.id },
    });
    postMessage({
      type: EVENT_NAMES_TO_CHAT.DONE_STREAMING,
      payload: { id: state.chat.id },
    });
  }, [postMessage, state.chat.id]);

  const hasContextFile = useMemo(() => {
    return state.chat.messages.some((message) =>
      isChatContextFileMessage(message),
    );
  }, [state.chat.messages]);

  const backFromChat = useCallback(() => {
    clearError();
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.BACK_FROM_CHAT,
      payload: { id: state.chat.id },
    });
  }, [clearError, postMessage, state.chat.id]);

  const openChatInNewTab = useCallback(() => {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.OPEN_IN_CHAT_IN_TAB,
      payload: { id: state.chat.id },
    });
  }, [postMessage, state.chat.id]);

  const sendToSideBar = useCallback(() => {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.SEND_TO_SIDE_BAR,
      payload: { id: state.chat.id },
    });
  }, [postMessage, state.chat.id]);

  const sendReadyMessage = useCallback(() => {
    const action: ReadyMessage = {
      type: EVENT_NAMES_FROM_CHAT.READY,
      payload: { id: state.chat.id },
    };
    postMessage(action);
  }, [postMessage, state.chat.id]);

  const handleNewFileClick = useCallback(
    (value: string) => {
      const action: NewFileFromChat = {
        type: EVENT_NAMES_FROM_CHAT.NEW_FILE,
        payload: {
          id: state.chat.id,
          content: value,
        },
      };

      postMessage(action);
    },
    [postMessage, state.chat.id],
  );

  const handlePasteDiffClick = useCallback(
    (value: string) => {
      const action: PasteDiffFromChat = {
        type: EVENT_NAMES_FROM_CHAT.PASTE_DIFF,
        payload: { id: state.chat.id, content: value },
      };
      postMessage(action);
    },
    [postMessage, state.chat.id],
  );

  // TODO: hoist this hook to context so useCallback isn't  needed
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const requestCommandsCompletion = useCallback(
    useDebounceCallback(
      function (
        query: string,
        cursor: number,
        trigger: string | null,
        // eslint-disable-next-line @typescript-eslint/no-inferrable-types
        number: number = 5,
      ) {
        const action: RequestAtCommandCompletion = {
          type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION,
          payload: { id: state.chat.id, query, cursor, trigger, number },
        };
        postMessage(action);
      },
      500,
      { leading: true },
    ),
    [state.chat.id],
  );

  const setSelectedCommand = useCallback(
    (command: string) => {
      const action: SetSelectedAtCommand = {
        type: EVENT_NAMES_TO_CHAT.SET_SELECTED_AT_COMMAND,
        payload: { id: state.chat.id, command },
      };
      dispatch(action);
    },
    [state.chat.id],
  );

  const removePreviewFileByName = useCallback(
    (name: string) => {
      const action: RemovePreviewFileByName = {
        type: EVENT_NAMES_TO_CHAT.REMOVE_PREVIEW_FILE_BY_NAME,
        payload: { id: state.chat.id, name },
      };

      dispatch(action);
    },
    [state.chat.id],
  );

  const retryQuestion = useCallback(
    (messages: ChatMessages) => {
      // set last_messages_length to messages.lent - 1
      const setMessageLengthAction: setPreviousMessagesLength = {
        type: EVENT_NAMES_TO_CHAT.SET_PREVIOUS_MESSAGES_LENGTH,
        payload: {
          id: state.chat.id,
          message_length: messages.length > 0 ? messages.length - 1 : 0,
        },
      };

      dispatch(setMessageLengthAction);
      sendMessages(messages, false);
    },
    [sendMessages, state.chat.id],
  );

  const startNewChat = useCallback(() => {
    const saveMessage: SaveChatFromChat = {
      type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT,
      payload: state.chat,
    };

    if (state.chat.messages.length > 0) {
      postMessage(saveMessage);
    }

    const message: CreateNewChatThread = {
      type: EVENT_NAMES_TO_CHAT.NEW_CHAT,
      payload: { id: state.chat.id },
    };
    dispatch(message);
  }, [postMessage, state.chat]);

  const setSelectedSystemPrompt = useCallback(
    (prompt: string) => {
      const action: SetSelectedSystemPrompt = {
        type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SYSTEM_PROMPT,
        payload: { id: state.chat.id, prompt },
      };
      dispatch(action);
    },
    [dispatch, state.chat.id],
  );

  useEffect(() => {
    sendReadyMessage();
  }, [sendReadyMessage]);

  // console.log({ state });

  return {
    state,
    askQuestion,
    clearError,
    setChatModel,
    stopStreaming,
    hasContextFile,
    backFromChat,
    openChatInNewTab,
    sendToSideBar,
    handleNewFileClick,
    handlePasteDiffClick,
    requestCommandsCompletion,
    setSelectedCommand,
    removePreviewFileByName,
    retryQuestion,
    maybeRequestCaps,
    startNewChat,
    setSelectedSystemPrompt,
  };
};
