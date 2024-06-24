import { useEffect, useReducer, useCallback, useMemo, useRef } from "react";
import {
  type ChatContextFile,
  type ChatMessages,
  type ChatResponse,
  isChatContextFileMessage,
  isChatContextFileDelta,
  isAssistantMessage,
  isAssistantDelta,
  isToolCallDelta,
  isToolResponse,
  isChatResponseChoice,
  ToolCommand,
  CodeChatModel,
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
  RequestPreviewFiles,
  type CommandCompletionResponse,
  type ToolResult,
  isSetTakeNotes,
  SetTakeNotes,
  TakeNotesFromChat,
  RequestTools,
  isRecieveTools,
  SetUseTools,
  QuestionFromChat,
  isSetUseTools,
  SetEnableSend,
  isSetEnableSend,
} from "../../events";
import { usePostMessage } from "../usePostMessage";
import { useDebounceCallback } from "usehooks-ts";
import { TAKE_NOTE_MESSAGE, mergeToolCalls } from "./utils";

export function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  if (isChatUserMessageResponse(response)) {
    if (response.role === "context_file") {
      return [...messages, [response.role, JSON.parse(response.content)]];
    } else if (response.role === "context_memory") {
      return [...messages, [response.role, JSON.parse(response.content)]];
    }
    return [...messages, [response.role, response.content]];
  }

  if (isToolResponse(response)) {
    const { tool_call_id, content, finish_reason } = response;
    const toolResult: ToolResult = { tool_call_id, content, finish_reason };
    return [...messages, [response.role, toolResult]];
  }

  if (!isChatResponseChoice(response)) {
    // console.log("Not a good response");
    // console.log(response);
    return messages;
  }

  return response.choices.reduce<ChatMessages>((acc, cur) => {
    if (isChatContextFileDelta(cur.delta)) {
      return acc.concat([[cur.delta.role, cur.delta.content]]);
    }

    if (
      acc.length === 0 &&
      "content" in cur.delta &&
      typeof cur.delta.content === "string" &&
      cur.delta.role
    ) {
      if (cur.delta.role === "assistant") {
        return acc.concat([
          [cur.delta.role, cur.delta.content, cur.delta.tool_calls],
        ]);
      }
      return acc.concat([cur.delta.role, cur.delta.content]);
    }

    const lastMessage = acc[acc.length - 1];

    if (isToolCallDelta(cur.delta)) {
      if (!isAssistantMessage(lastMessage)) {
        return acc.concat([
          ["assistant", cur.delta.content ?? "", cur.delta.tool_calls],
        ]);
      }

      const last = acc.slice(0, -1);
      const collectedCalls = lastMessage[2] ?? [];
      const calls = mergeToolCalls(collectedCalls, cur.delta.tool_calls);
      const content = cur.delta.content;
      const message = content ? lastMessage[1] + content : lastMessage[1];

      return last.concat([["assistant", message, calls]]);
    }

    if (
      isAssistantMessage(lastMessage) &&
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      const last = acc.slice(0, -1);
      const currentMessage = lastMessage[1] ?? "";
      const toolCalls = lastMessage[2];
      return last.concat([
        ["assistant", currentMessage + cur.delta.content, toolCalls],
      ]);
    } else if (
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      return acc.concat([["assistant", cur.delta.content]]);
    } else if (cur.delta.role === "assistant") {
      // empty message from JB
      return acc;
    }

    if (cur.delta.role === null || cur.finish_reason !== null) {
      return acc;
    }

    // console.log("Fall though");
    // console.log({ cur, lastMessage });

    return acc;
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
      // TODO: set use_tool
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

      const lastAssistantMessage = messages.reduce((count, message, index) => {
        if (message[0] === "assistant") return index + 1;
        return count;
      }, 0);

      return {
        ...state,
        waiting_for_response: false,
        prevent_send: true,
        streaming: false,
        error: null,
        previous_message_length: lastAssistantMessage,
        chat: {
          ...action.payload.chat,
          messages,
        },
        selected_snippet: action.payload.snippet ?? state.selected_snippet,
        take_notes: false,
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
      // TODO: check caps that the model supports tools
      const default_cap = action.payload.caps.code_chat_default_model;
      const available_caps = action.payload.caps.code_chat_models;
      const cap_names = Object.keys(available_caps);
      const error = cap_names.length === 0 ? "No available caps" : null;
      // const cap = state.chat.model || default_cap
      // const tools = available_caps[cap].supports_tools

      return {
        ...state,
        error,
        caps: {
          fetching: false,
          default_cap: default_cap || cap_names[0] || "",
          available_caps,
          error: null,
        },
      };
    }

    if (isThisChat && isChatReceiveCapsError(action)) {
      const error =
        state.error === null && state.caps.error === null
          ? action.payload.message
          : state.error;
      return {
        ...state,
        error: error,
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
        prevent_send: false,
        waiting_for_response: false,
        streaming: false,
      };
    }

    if (isThisChat && isChatErrorStreaming(action)) {
      return {
        ...state,
        streaming: false,
        prevent_send: true,
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
      return {
        ...state,
        commands: {
          completions: action.payload.completions,
          replace: action.payload.replace,
          is_cmd_executable: action.payload.is_cmd_executable,
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

    if (isThisChat && isSetTakeNotes(action)) {
      return {
        ...state,
        take_notes: action.payload.take_notes,
      };
    }

    if (isThisChat && isRecieveTools(action)) {
      const { tools } = action.payload;
      if (tools.length === 0) {
        return {
          ...state,
          tools: [],
          use_tools: false,
        };
      }

      return {
        ...state,
        tools,
      };
    }

    if (isThisChat && isSetUseTools(action)) {
      return {
        ...state,
        use_tools: action.payload.use_tools,
      };
    }

    if (isThisChat && isSetEnableSend(action)) {
      return {
        ...state,
        prevent_send: !action.payload.enable_send,
      };
    }

    return state;
  };
}

export type ChatCapsState = {
  fetching: boolean;
  default_cap: string;
  // available_caps: string[];
  available_caps: Record<string, CodeChatModel>;
  error: null | string;
};

export type ChatState = {
  chat: ChatThread;
  prevent_send: boolean;
  waiting_for_response: boolean;
  streaming: boolean;
  previous_message_length: number;
  error: string | null;
  caps: ChatCapsState;
  commands: CommandCompletionResponse;
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
  take_notes: boolean;
  // Check caps if model has tools
  tools: ToolCommand[] | null;
  use_tools: boolean;
};

export function createInitialState(): ChatState {
  return {
    streaming: false,
    prevent_send: false,
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
      available_caps: {},
      error: null,
    },
    commands: {
      completions: [],
      replace: [-1, -1],
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
    take_notes: true,
    tools: null,
    use_tools: false,
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
  }, [dispatch]);

  const clearError = useCallback(() => {
    dispatch({
      type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR,
      payload: { id: state.chat.id },
    });
  }, [state.chat.id]);

  const setTakeNotes = useCallback(
    (take_notes: boolean) => {
      const action: SetTakeNotes = {
        type: EVENT_NAMES_TO_CHAT.SET_TAKE_NOTES,
        payload: { id: state.chat.id, take_notes },
      };

      dispatch(action);
    },
    [state.chat.id],
  );

  const maybeDefaultPrompt: string | null = useMemo(() => {
    return "default" in state.system_prompts.prompts
      ? state.system_prompts.prompts.default.text
      : null;
  }, [state.system_prompts.prompts]);

  const sendMessages = useCallback(
    (messages: ChatMessages, attach_file = state.active_file.attach) => {
      clearError();
      // setTakeNotes(true);
      dispatch({
        type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT,
        payload: { id: state.chat.id, disable: true },
      });

      const messagesWithSystemPrompt: ChatMessages =
        state.selected_system_prompt &&
        state.selected_system_prompt !== maybeDefaultPrompt
          ? [["system", state.selected_system_prompt], ...messages]
          : messages;

      const thread: ChatThread = {
        id: state.chat.id,
        messages: messagesWithSystemPrompt,
        title: state.chat.title,
        model: state.chat.model,
        attach_file,
      };

      dispatch({
        type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES,
        payload: thread,
      });

      const action: QuestionFromChat = {
        type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
        payload: { ...thread, tools: state.use_tools ? state.tools : null },
      };

      postMessage(action);

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
      state.active_file.attach,
      state.chat.id,
      state.chat.title,
      state.chat.model,
      state.selected_system_prompt,
      state.use_tools,
      state.tools,
      clearError,
      maybeDefaultPrompt,
      postMessage,
    ],
  );

  const askQuestion = useCallback(
    (question: string) => {
      const messages = state.chat.messages.concat([["user", question]]);

      sendMessages(messages);
    },
    [sendMessages, state.chat.messages],
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
    const caps = Object.keys(state.caps.available_caps);
    if (
      state.chat.messages.length === 0 &&
      caps.length === 0 &&
      !state.caps.fetching
    ) {
      requestCaps();
    }
  }, [
    state.caps.available_caps,
    state.caps.fetching,
    state.chat.messages.length,
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
    setTakeNotes(true);

    postMessage({
      type: EVENT_NAMES_FROM_CHAT.OPEN_IN_CHAT_IN_TAB,
      payload: { id: state.chat.id },
    });
  }, [postMessage, state.chat.id, setTakeNotes]);

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

  // TODO: hoist this hook to context so useCallback isn't needed
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const requestCommandsCompletion = useCallback(
    useDebounceCallback(
      (
        query: string,
        cursor: number,
        // eslint-disable-next-line @typescript-eslint/no-inferrable-types
        number: number = 5,
      ) => {
        const action: RequestAtCommandCompletion = {
          type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION,
          payload: { id: state.chat.id, query, cursor, number },
        };
        postMessage(action);
      },
      500,
      { leading: true, maxWait: 250 },
    ),
    [state.chat.id, postMessage],
  );

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const requestPreviewFiles = useCallback(
    useDebounceCallback(
      function (input: string) {
        const message: RequestPreviewFiles = {
          type: EVENT_NAMES_FROM_CHAT.REQUEST_PREVIEW_FILES,
          payload: { id: state.chat.id, query: input },
        };
        postMessage(message);
      },
      500,
      { leading: true },
    ),
    [postMessage, state.chat.id],
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

  useEffect(() => {
    if (
      !state.streaming &&
      state.chat.messages.length > 0 &&
      !state.error &&
      !state.prevent_send
    ) {
      const lastMessage = state.chat.messages[state.chat.messages.length - 1];
      if (
        isAssistantMessage(lastMessage) &&
        lastMessage[2] &&
        lastMessage[2].length > 0
      ) {
        sendMessages(state.chat.messages);
      }
    }
  }, [
    sendMessages,
    state.chat.messages,
    state.streaming,
    state.error,
    state.prevent_send,
  ]);

  // TODO: Turn this into a hook
  const noteRef = useRef<Pick<ChatState, "chat" | "take_notes">>({
    chat: state.chat,
    take_notes: state.take_notes,
  });
  useEffect(() => {
    noteRef.current.chat = state.chat;
    noteRef.current.take_notes = state.take_notes;
  }, [state.chat, state.take_notes]);

  useEffect(() => {
    return () => {
      // the clean up function is called when the component unmounts (chat is closed)
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const { chat, take_notes } = noteRef.current;
      if (!take_notes) return;
      if (chat.messages.length === 0) return;

      const messages: ChatMessages = [
        ...chat.messages,
        ["user", TAKE_NOTE_MESSAGE],
      ];

      const action: TakeNotesFromChat = {
        type: EVENT_NAMES_FROM_CHAT.TAKE_NOTES,
        payload: { ...chat, messages },
      };

      postMessage(action);
    };
  }, [postMessage, state.chat.id]);

  const requestTools = useCallback(() => {
    const action: RequestTools = {
      type: EVENT_NAMES_FROM_CHAT.REQUEST_TOOLS,
      payload: { id: state.chat.id },
    };
    postMessage(action);
  }, [postMessage, state.chat.id]);

  useEffect(() => {
    requestTools();
  }, [requestTools]);

  const setUseTools = useCallback(
    (value: boolean) => {
      const action: SetUseTools = {
        type: EVENT_NAMES_TO_CHAT.SET_USE_TOOLS,
        payload: { id: state.chat.id, use_tools: value },
      };
      dispatch(action);
    },
    [state.chat.id],
  );

  const enableSend = useCallback(
    (value: boolean) => {
      const action: SetEnableSend = {
        type: EVENT_NAMES_TO_CHAT.SET_ENABLE_SEND,
        payload: { id: state.chat.id, enable_send: value },
      };

      dispatch(action);
    },
    [state.chat.id],
  );

  // useEffect(() => {
  //   window.debugChat =
  //     window.debugChat ||
  //     function () {
  //       console.log(state.chat);
  //     };

  //   return () => {
  //     window.debugChat = undefined;
  //   };
  // }, [state.chat]);

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
    requestPreviewFiles,
    setUseTools,
    enableSend,
  };
};
