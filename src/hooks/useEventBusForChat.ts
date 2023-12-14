import { useEffect, useReducer } from "react";
import { ChatMessages, ChatResponse } from "../services/refact";
import { v4 as uuidv4 } from "uuid";
import { EVENT_NAMES_TO_CHAT, EVENT_NAMES_FROM_CHAT } from "../events";


interface BaseAction {
  type: string;
  payload: unknown;
}

interface MessageFromChat extends BaseAction {
  type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE //"chat_response"; // TODO: using a constant didn't work, use enum instead
  payload: ChatResponse;
}

interface BackUpMessages extends BaseAction {
  type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES; // "back_up_messages";
  payload: ChatMessages;
}

interface RestoreChat extends BaseAction {
  type:  EVENT_NAMES_TO_CHAT.RESTORE_CHAT //"restore_chat_from_history";
  payload: ChatState;
}

interface NewChatThread extends BaseAction {
  type: EVENT_NAMES_TO_CHAT.NEW_CHAT;
}

type Actions = MessageFromChat | BackUpMessages | RestoreChat | NewChatThread;

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

function reducer(state: ChatState, action: Actions) {

  switch (action.type) {
    case EVENT_NAMES_TO_CHAT.CHAT_RESPONSE: {
      if (action.payload.id !== state.id) return state;
      const messages = formatChatResponse(state.messages, action.payload);
      return {
        ...state,
        messages,
      };
    }
    case EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES: {
      return {
        ...state,
        messages: action.payload,
      };
    }

    case EVENT_NAMES_TO_CHAT.RESTORE_CHAT: {
      return {
        ...state,
        ...action.payload,
      };
    }

    case EVENT_NAMES_TO_CHAT.NEW_CHAT: {
      return initialState;
    }

    default:
      return state;
  }
}

export type ChatState = {
  id: string;
  messages: ChatMessages;
  title?: string;
  model: string;
};

const initialState: ChatState = {
  id: uuidv4(),
  messages: [],
  title: "",
  model: "gpt-3.5-turbo",
};

export const useEventBusForChat = () => {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      if (event.source !== window) {
        return;
      }
      // TODO: validate events
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      switch (event.data.type) {
        case  EVENT_NAMES_TO_CHAT.DONE_STREAMING: {
          postMessage({
            type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT,
            payload: state,
          });
          return;
        }
        default:
          dispatch(event.data as Actions);
      }
    };

    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, [state, dispatch]);

  function askQuestion(question: string) {
    const messagesToSend = state.messages.concat([["user", question]]);

    dispatch({ type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES, payload: messagesToSend });
    postMessage({
      type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
      payload: {
        id: state.id,
        messages: messagesToSend,
        title: state.title,
        model: state.model,
      },
    });
  }

  return { state, askQuestion };
};
