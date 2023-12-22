import { useEffect, useReducer } from "react";
import { ChatMessages, ChatResponse } from "../services/refact";
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
} from "../events";

declare global {
  interface Window {
    postIntellijMessage?(message: Record<string, unknown>): void;
    acquireVsCodeApi?(): {
      postMessage: (message: Record<string, unknown>) => void;
    };
  }
}

function postMessage(message: Record<string, unknown>) {
  const vscode = window.acquireVsCodeApi ? window.acquireVsCodeApi() : null;
  if (vscode) {
    vscode.postMessage(message);
  } else if (window.postIntellijMessage) {
    window.postIntellijMessage(message);
  } else {
    window.postMessage(message, "*");
  }
}

function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  return response.choices.reduce<ChatMessages>((acc, cur) => {
    if (cur.delta.role === "context_file") {
      return acc.concat([[cur.delta.role, cur.delta.file_content || ""]]);
    }
    if (acc.length === 0) {
      return acc.concat([[cur.delta.role, cur.delta.content]]);
    }
    const lastMessage = acc[acc.length - 1];

    if (lastMessage[0] === cur.delta.role) {
      const head = acc.slice(0, -1);
      return head.concat([
        [cur.delta.role, lastMessage[1] + cur.delta.content],
      ]);
    }

    return acc.concat([[cur.delta.role, cur.delta.content]]);
  }, messages);
}

function reducer(state: ChatState, action: ActionToChat): ChatState {
  const isThisChat =
    action.payload?.id && action.payload.id === state.chat.id ? true : false;

  if (isThisChat && isSetDisableChat(action)) {
    return { ...state, streaming: action.payload.disable };
  }

  if (isThisChat && isResponseToChat(action)) {
    const messages = formatChatResponse(state.chat.messages, action.payload);
    return {
      ...state,
      streaming: true,
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
    return {
      ...state,
      streaming: false,
      error: null,
      chat: action.payload,
    };
  }

  if (isCreateNewChat(action)) {
    return createInitialState();
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

    return {
      ...state,
      chat: {
        ...state.chat,
        model: state.chat.model || default_cap,
      },
      caps: {
        fetching: false,
        default_cap,
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
    // note: should avoid side effects in reducer :/
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT,
      payload: state.chat,
    });

    return {
      ...state,
      streaming: false,
    };
  }

  if (isThisChat && isChatErrorStreaming(action)) {
    return {
      ...state,
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

  return state;
}

export type ChatCapsState = {
  fetching: boolean;
  default_cap: string;
  available_caps: string[];
};

export type ChatState = {
  chat: ChatThread;
  streaming: boolean;
  error: string | null;
  caps: ChatCapsState;
};

function createInitialState(): ChatState {
  return {
    streaming: false,
    error: null,
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
  };
}

const initialState = createInitialState();

export const useEventBusForChat = () => {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }
      if (isActionToChat(event.data)) {
        dispatch(event.data);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [state, dispatch]);

  function askQuestion(question: string) {
    dispatch({
      type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR,
      payload: { id: state.chat.id },
    });
    dispatch({
      type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT,
      payload: { id: state.chat.id, disable: true },
    });
    const messages = state.chat.messages.concat([["user", question]]);
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
    const payload = {
      id: state.chat.id,
      messages: messages,
      title: state.chat.title,
      model: state.chat.model,
    };

    dispatch({
      type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES,
      payload,
    });
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
      payload,
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
      state.caps.fetching
    ) {
      requestCaps();
    }
  }, [state]);

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

  return {
    state,
    askQuestion,
    sendMessages,
    clearError,
    setChatModel,
    stopStreaming,
  };
};
