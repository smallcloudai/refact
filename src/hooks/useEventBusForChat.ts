import { useEffect, useReducer } from "react";
import { ChatMessages, ChatChoice, ChatResponse } from "../services/refact";


const CHAT_TYPE = "chat"; // TODO: make this longer


// const ADD_MESSAGE = "add_message";
// type AddMessage = {
//     type: ADD_MESSAGE;
//     payload: CHAT_MESSAGE;
// }

// type Actions = MessageFromChat
interface BaseAction {
  type: string;
  payload: unknown;
};

interface MessageFromChat extends BaseAction {
  type: "chat";
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
console.log("reducer");
console.log({ state, action });
  switch(action.type) {
    case CHAT_TYPE: {
      // return {
      //   ...state,
      //   choices: state.choices.concat(action.payload.choices)
      // };
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
    choices: ChatChoice[];
    messages: ChatMessages;
}

const initialState: ChatState = {
    choices: [],
    messages: [],
}

function postChatToHost(question: string, messages: ChatMessages) {
    const messagesToSend = messages.concat([
        ["user", question]
    ])

    postMessage({type: "chat_question", payload: messagesToSend});
    return messagesToSend;
}


export const useEventBusForChat = () => {
  const [state, dispatch] = useReducer(reducer, initialState);
  console.log({state});
  // // this isn't good
  // const messages = state.choices.reduce<ChatMessages>((acc, cur) => {
  //   if(cur.delta.role === "context_file") {
  //       return acc.concat([[cur.delta.role, cur.delta.file_content || ""]])
  //   }
  //   if(acc.length === 0) {
  //       return acc.concat([[cur.delta.role, cur.delta.content]])
  //   }
  //   const lastMessage = acc[acc.length - 1];
  //   if(lastMessage[0] === cur.delta.role) {
  //       const head = acc.slice(0, -1)
  //       return head.concat([[cur.delta.role, lastMessage[1] + cur.delta.content]])
  //   }

  //   return acc.concat([[cur.delta.role, cur.delta.content]])
  // }, [])

  const listener = (event: MessageEvent) => {
    console.log(event);
    // check valid event from window
    // validate the payload
    dispatch(event.data as Actions);
  };

  useEffect(() => {
    window.addEventListener("message", listener);

    return () => {
      window.removeEventListener("message", listener);
    };
  }, []);

  function askQuestion(question: string) {

    const newMessages = postChatToHost(question, state.messages);
    dispatch({type: "back_up_messages", payload: newMessages})
  }

  return { state, askQuestion };
}
