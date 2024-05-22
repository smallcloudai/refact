import {
  AssistantMessage,
  ChatMessage,
  ChatMessages,
  ToolCallsMessage,
} from "../../events";

function isAssistantMessage(message: ChatMessage): message is AssistantMessage {
  return message[0] === "assistant";
}

function isToolCallsMesage(message: ChatMessage): message is ToolCallsMessage {
  return message[0] === "tool_calls";
}

export function appendToolCallsToAssistantMessage(
  messages: ChatMessages,
): ChatMessages {
  return messages.reduce<ChatMessages>((acc, message) => {
    if (isToolCallsMesage(message)) {
      const toolCalls = message[1];
      for (let i = acc.length - 1; i >= 0; i--) {
        const lastMessage = acc[i];
        if (isAssistantMessage(lastMessage)) {
          if (lastMessage[2] !== undefined) {
            lastMessage[2] = [...lastMessage[2], ...toolCalls];
          } else {
            lastMessage[2] = toolCalls;
          }
          return acc;
        }
      }
    }

    return acc.concat([message]);
  }, []);
}
