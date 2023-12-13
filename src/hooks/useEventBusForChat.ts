import { useEffect, useReducer } from "react";
import { ChatMessages, ChatResponse } from "../services/refact";


const CHAT_TYPE = "chat_response"; // TODO: make this longer

interface BaseAction {
  type: string;
  payload: unknown;
};

interface MessageFromChat extends BaseAction {
  type:  "chat_response"; // CHAT_TYPE didn't work
  payload: ChatResponse
}

interface BackUpMessages extends BaseAction {
  type: "back_up_messages";
  payload: ChatMessages;
}

type Actions = MessageFromChat | BackUpMessages;

declare global {
    interface Window {
        postIntellijMessage?(message: Record<string, unknown>): void;
        acquireVsCodeApi?(): {
            postMessage: (message: Record<string, unknown>) => void;
        }
    }
}

function postMessage(message: Record<string, unknown>) {
    const vscode = window.acquireVsCodeApi ? window.acquireVsCodeApi() : null
    if(vscode) {
        vscode.postMessage(message);
    } else if(window.postIntellijMessage){
        window.postIntellijMessage(message);
    } else {
        window.postMessage(message, "*");
    }
}

function formatChatResponse(messages: ChatMessages, response: ChatResponse): ChatMessages {
    return response.choices.reduce<ChatMessages>((acc, cur) => {

      if(cur.delta.role === "context_file") {
          return acc.concat([[cur.delta.role, cur.delta.file_content || ""]])
      }
      if(acc.length === 0) {
          return acc.concat([[cur.delta.role, cur.delta.content]])
      }
      const lastMessage = acc[acc.length - 1];
      console.log({lastMessage, cur})
      if(lastMessage[0] === cur.delta.role) {
          const head = acc.slice(0, -1)
          return head.concat([[cur.delta.role, lastMessage[1] + cur.delta.content]])
      }

      return acc.concat([[cur.delta.role, cur.delta.content]])
    }, messages)
}

function reducer(state: ChatState, action: Actions) { // TODO: action types

  switch(action.type) {
    case CHAT_TYPE: {
      const messages = formatChatResponse(state.messages, action.payload)
      return {
        ...state,
        messages
      }
    }
    case "back_up_messages": {
      return {
        ...state,
        messages: action.payload,
      }
    }
    default: return state
  }

}

type ChatState = {
    messages: ChatMessages;
}

const initialState: ChatState = {
    messages: [],
}


export const useEventBusForChat = () => {
  const [state, dispatch] = useReducer(reducer, initialState);

  const listener = (event: MessageEvent) => {
    dispatch(event.data as Actions);
  };

  useEffect(() => {
    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, []);

  function askQuestion(question: string) {
    const messagesToSend = state.messages.concat([
      ["user", question]
  ])

    dispatch({type: "back_up_messages", payload: messagesToSend});
    postMessage({type: "chat_question", payload: messagesToSend});
  }

  return { state, askQuestion };
}
