import { useEffect, useReducer } from "react";
import { ChatMessages, ChatResponse } from "../services/refact";
import { v4 as uuidv4 } from "uuid";

const CHAT_TYPE = "chat_response"; // TODO: make this longer

interface BaseAction {
  type: string;
  payload: unknown;
}

interface MessageFromChat extends BaseAction {
  type: "chat_response"; // TODO: using a constant didn't work, use enum instead
  payload: ChatResponse;
}

interface BackUpMessages extends BaseAction {
  type: "back_up_messages";
  payload: ChatMessages;
}

interface RestoreChat extends BaseAction {
  type: "restore_chat_from_history";
  payload: ChatState;
}

type Actions = MessageFromChat | BackUpMessages | RestoreChat;

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
    case CHAT_TYPE: {
      if (action.payload.id !== state.id) return state;
      const messages = formatChatResponse(state.messages, action.payload);
      return {
        ...state,
        messages,
      };
    }
    case "back_up_messages": {
      return {
        ...state,
        messages: action.payload,
      };
    }

    case "restore_chat_from_history": {
      return {
        ...state,
        ...action.payload,
      };
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
        case "chat_done_streaming": {
          console.log("client saving chat");
          console.log(state);
          postMessage({
            type: "save_chat_to_history",
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

    dispatch({ type: "back_up_messages", payload: messagesToSend });
    postMessage({
      type: "chat_question",
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
