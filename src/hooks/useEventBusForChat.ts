import { useEffect, useReducer } from "react";
import { ChatMessages, ChatResponse } from "../services/refact";
import { v4 as uuidv4 } from "uuid";
import {
  EVENT_NAMES_TO_CHAT,
  EVENT_NAMES_FROM_CHAT,
  Actions,
  ChatThread,
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

function reducer(state: ChatState, action: Actions): ChatState {
  switch (action.type) {
    case EVENT_NAMES_TO_CHAT.CHAT_RESPONSE: {
      if (action.payload.id !== state.chat.id) return state;
      const messages = formatChatResponse(state.chat.messages, action.payload);
      return {
        ...state,
        chat: {
          ...state.chat,
          messages,
        },
      };
    }
    case EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES: {
      return {
        ...state,
        chat: {
          ...state.chat,
          messages: action.payload,
        },
      };
    }

    case EVENT_NAMES_TO_CHAT.RESTORE_CHAT: {
      return {
        ...state,
        streaming: false,
        chat: action.payload,
      };
    }

    case EVENT_NAMES_TO_CHAT.NEW_CHAT: {
      return createInitialState();
    }

    case EVENT_NAMES_TO_CHAT.DONE_STREAMING: {
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

    default:
      return state;
  }
}

export type ChatState = {
  chat: ChatThread;
  streaming: boolean;
};

function createInitialState(): ChatState {
  return {
    streaming: false,
    chat: {
      id: uuidv4(),
      messages: [],
      title: "",
      model: "gpt-3.5-turbo",
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
      // TODO: validate events

      dispatch(event.data as Actions);
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [state, dispatch]);

  function askQuestion(question: string) {
    const messagesToSend = state.chat.messages.concat([["user", question]]);

    dispatch({
      type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES,
      payload: messagesToSend,
    });
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
      payload: {
        id: state.chat.id,
        messages: messagesToSend,
        title: state.chat.title,
        model: state.chat.model,
      },
    });
  }

  return { state, askQuestion };
};
