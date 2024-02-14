import { useEffect, useReducer, useCallback } from "react";
import {
  ChatContextFile,
  ChatContextFileMessage,
  ChatMessages,
  ChatResponse,
  isChatContextFileMessage,
} from "../services/refact";
import { v4 as uuidv4 } from "uuid";
import {
  EVENT_NAMES_TO_CHAT,
  EVENT_NAMES_FROM_CHAT,
  isActionToChat,
  ActionToChat,
  ChatThread,
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
  isReceiveContextFile,
  isRequestForFileFromChat,
  isRemoveContext,
  isActiveFileInfo,
  isToggleActiveFile,
  ToggleActiveFile,
  NewFileFromChat,
  PasteDiffFromChat,
  ReadyMessage,
  RequestAtCommandCompletion,
  isReceiveAtCommandCompletion,
  SetSelectedAtCommand,
  isSetSelectedAtCommand,
  RequestAtCommandPreview,
  isReceiveAtCommandPreview,
  isRemoveLastUserMessage,
  isChatUserMessageResponse,
  isChatSetLastModelUsed,
  isSetSelectedSnippet,
  isRemovePreviewFileByName,
  RemovePreviewFileByName,
} from "../events";
import { useConfig } from "../contexts/config-context";
import { usePostMessage } from "./usePostMessage";
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

function reducer(state: ChatState, action: ActionToChat): ChatState {
  const isThisChat =
    action.payload?.id && action.payload.id === state.chat.id ? true : false;

  if (isThisChat && isSetDisableChat(action)) {
    return {
      ...state,
      streaming: action.payload.disable,
      waiting_for_response: action.payload.disable,
    };
  }

  if (isThisChat && isResponseToChat(action)) {
    const messages = formatChatResponse(state.chat.messages, action.payload);
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

  if (!isThisChat && isRestoreChat(action)) {
    const messages: ChatMessages = action.payload.messages.map((message) => {
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
    });
    return {
      ...state,
      waiting_for_response: false,
      streaming: false,
      error: null,
      previous_message_length: messages.length,
      chat: {
        ...action.payload,
        messages,
      },
    };
  }

  if (isCreateNewChat(action)) {
    const nextState = createInitialState();

    return {
      ...nextState,
      chat: {
        ...nextState.chat,
        model: state.chat.model,
      },
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
      chat: {
        ...state.chat,
        model: state.chat.model || default_cap,
      },
      caps: {
        fetching: false,
        default_cap: default_cap || available_caps[0] || "",
        available_caps,
      },
    };
  }

  if (isThisChat && isChatReceiveCapsError(action)) {
    return {
      ...state,
      error: action.payload.message,
      caps: {
        ...state.caps,
        fetching: false,
      },
    };
  }

  if (isThisChat && isChatDoneStreaming(action)) {
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
      error: action.payload.message,
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

  if (isThisChat && isRequestForFileFromChat(action)) {
    return {
      ...state,
      waiting_for_response: true,
    };
  }

  if (isThisChat && isReceiveContextFile(action)) {
    return {
      ...state,
      waiting_for_response: false,
      chat: {
        ...state.chat,
        messages: [
          ["context_file", action.payload.files],
          ...state.chat.messages,
        ],
      },
    };
  }

  if (isThisChat && isRemoveContext(action)) {
    const messages = state.chat.messages.filter(
      (message) => !isChatContextFileMessage(message),
    );

    return {
      ...state,
      chat: {
        ...state.chat,
        messages,
      },
    };
  }

  if (isThisChat && isActiveFileInfo(action)) {
    const { name, can_paste } = action.payload;
    return {
      ...state,
      active_file: {
        name,
        can_paste,
        attach: state.active_file.attach,
      },
    };
  }

  if (isThisChat && isToggleActiveFile(action)) {
    return {
      ...state,
      active_file: {
        ...state.active_file,
        attach: action.payload.attach_file,
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
      rag_commands: {
        ...state.rag_commands,
        selected_command: "",
        is_cmd_executable: false,
        available_commands: [],
      },
    };
  }

  if (isThisChat && isRemoveLastUserMessage(action)) {
    const messages = state.chat.messages.slice(
      0,
      state.previous_message_length,
    );
    return {
      ...state,
      chat: {
        ...state.chat,
        messages,
      },
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
      selected_snippet: {
        language: action.payload.language,
        code: action.payload.snippet,
      },
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

  return state;
}

export type ChatCapsState = {
  fetching: boolean;
  default_cap: string;
  available_caps: string[];
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
  active_file: {
    name: string;
    attach: boolean;
    can_paste: boolean;
  };
  selected_snippet: {
    language: string;
    code: string;
  };
};

function createInitialState(): ChatState {
  return {
    streaming: false,
    waiting_for_response: false,
    error: null,
    previous_message_length: 0,
    selected_snippet: {
      language: "",
      code: "",
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
    },
    rag_commands: {
      available_commands: [],
      selected_command: "",
      arguments: [],
      is_cmd_executable: false,
    },

    active_file: {
      name: "",
      attach: false,
      can_paste: false,
    },
  };
}

const initialState = createInitialState();
// Maybe use context to avoid prop drilling?
export const useEventBusForChat = () => {
  const config = useConfig();
  const [state, dispatch] = useReducer(reducer, initialState);
  const postMessage = usePostMessage();

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (isActionToChat(event.data)) {
        dispatch(event.data);
      }

      if (
        isActionToChat(event.data) &&
        event.data.payload?.id &&
        event.data.payload.id === state.chat.id &&
        isChatDoneStreaming(event.data)
      ) {
        postMessage({
          type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT,
          payload: state.chat,
        });
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [state, dispatch, postMessage]);

  function askQuestion(question: string) {
    const filesInPreview: ChatContextFileMessage[] =
      state.files_in_preview.length > 0
        ? [["context_file", state.files_in_preview]]
        : [];
    const messages = state.chat.messages
      .concat(filesInPreview)
      .concat([["user", question]]);
    sendMessages(messages);
  }

  function sendMessages(messages: ChatMessages) {
    dispatch({
      type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR,
      payload: { id: state.chat.id },
    });
    dispatch({
      type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT,
      payload: { id: state.chat.id, disable: true },
    });

    const payload: ChatThread = {
      id: state.chat.id,
      messages: messages,
      title: state.chat.title,
      model: state.chat.model,
      attach_file: state.active_file.attach,
    };

    dispatch({
      type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES,
      payload,
    });
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
      payload,
    });

    dispatch({
      type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SNIPPET,
      payload: { id: state.chat.id, snippet: "", language: "" },
    });
  }

  useEffect(() => {
    function requestCaps() {
      postMessage({
        type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS,
        payload: {
          id: state.chat.id,
        },
      });
    }

    if (
      state.chat.messages.length === 0 &&
      state.caps.available_caps.length === 0 &&
      !state.caps.fetching &&
      !state.error
    ) {
      requestCaps();
    }
  }, [state, postMessage]);

  function clearError() {
    dispatch({
      type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR,
      payload: { id: state.chat.id },
    });
  }

  function setChatModel(model: string) {
    const action = {
      type: EVENT_NAMES_TO_CHAT.SET_CHAT_MODEL,
      payload: {
        id: state.chat.id,
        model,
      },
    };
    dispatch(action);
  }

  function stopStreaming() {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.STOP_STREAMING,
      payload: { id: state.chat.id },
    });
  }

  const hasContextFile = state.chat.messages.some((message) =>
    isChatContextFileMessage(message),
  );

  function handleContextFileForWeb() {
    if (hasContextFile) {
      dispatch({
        type: EVENT_NAMES_TO_CHAT.REMOVE_FILES,
        payload: { id: state.chat.id },
      });
    } else {
      postMessage({
        type: EVENT_NAMES_FROM_CHAT.REQUEST_FILES,
        payload: { id: state.chat.id },
      });
    }
  }

  function handleContextFile(toggle?: boolean) {
    if (config.host === "web") {
      handleContextFileForWeb();
    } else {
      const action: ToggleActiveFile = {
        type: EVENT_NAMES_TO_CHAT.TOGGLE_ACTIVE_FILE,
        payload: { id: state.chat.id, attach_file: !!toggle },
      };
      dispatch(action);
    }
  }

  function backFromChat() {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.BACK_FROM_CHAT,
      payload: { id: state.chat.id },
    });
  }

  function openChatInNewTab() {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.OPEN_IN_CHAT_IN_TAB,
      payload: { id: state.chat.id },
    });
  }

  function sendToSideBar() {
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.SEND_TO_SIDE_BAR,
      payload: { id: state.chat.id },
    });
  }

  function sendReadyMessage() {
    const action: ReadyMessage = {
      type: EVENT_NAMES_FROM_CHAT.READY,
      payload: { id: state.chat.id },
    };
    postMessage(action);
  }

  function handleNewFileClick(value: string) {
    const action: NewFileFromChat = {
      type: EVENT_NAMES_FROM_CHAT.NEW_FILE,
      payload: {
        id: state.chat.id,
        content: value,
      },
    };

    postMessage(action);
  }

  function handlePasteDiffClick(value: string) {
    const action: PasteDiffFromChat = {
      type: EVENT_NAMES_FROM_CHAT.PASTE_DIFF,
      payload: { id: state.chat.id, content: value },
    };
    postMessage(action);
  }

  // TODO: hoise this hook to context so useCallback isn't  needed
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const requestCommandsCompletion = useCallback(
    useDebounceCallback(
      function (
        query: string,
        cursor: number,
        // eslint-disable-next-line @typescript-eslint/no-inferrable-types
        number: number = 5,
      ) {
        const action: RequestAtCommandCompletion = {
          type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION,
          payload: { id: state.chat.id, query, cursor, number },
        };
        postMessage(action);
      },
      500,
      { leading: true },
    ),
    [state.chat.id],
  );

  function setSelectedCommand(command: string) {
    const action: SetSelectedAtCommand = {
      type: EVENT_NAMES_TO_CHAT.SET_SELECTED_AT_COMMAND,
      payload: { id: state.chat.id, command },
    };
    dispatch(action);
  }

  const executeCommand = useDebounceCallback(
    (command: string, cursor: number) => {
      const action: RequestAtCommandPreview = {
        type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_PREVIEW,
        payload: { id: state.chat.id, query: command, cursor },
      };
      if (!state.chat.model) {
        setChatModel(state.caps.default_cap);
      }
      postMessage(action);
    },
    500,
    { leading: true },
  );

  function removePreviewFileByName(name: string) {
    const action: RemovePreviewFileByName = {
      type: EVENT_NAMES_TO_CHAT.REMOVE_PREVIEW_FILE_BY_NAME,
      payload: { id: state.chat.id, name },
    };

    dispatch(action);
  }

  return {
    state,
    askQuestion,
    sendMessages,
    clearError,
    setChatModel,
    stopStreaming,
    handleContextFile,
    hasContextFile,
    backFromChat,
    openChatInNewTab,
    sendToSideBar,
    sendReadyMessage,
    handleNewFileClick,
    handlePasteDiffClick,
    requestCommandsCompletion,
    setSelectedCommand,
    executeCommand,
    removePreviewFileByName,
  };
};
