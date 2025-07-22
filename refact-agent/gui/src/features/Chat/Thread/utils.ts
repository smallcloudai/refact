import {
  ChatMessages,
  isAssistantMessage,
  isDiffMessage,
  isToolMessage,
  isUserMessage,
  LSPUserMessage,
} from "../../../services/refact";
import { type LspChatMessage } from "../../../services/refact";

export function formatMessagesForLsp(messages: ChatMessages): LspChatMessage[] {
  return messages.reduce<LspChatMessage[]>((acc, message) => {
    if (isUserMessage(message)) {
      const { ftm_role, ftm_content, ...rest } = message;
      const msg: LSPUserMessage = {
        ...rest,
        role: ftm_role,
        content: ftm_content,
      };
      return acc.concat([msg]);
    }

    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message.ftm_role,
          content: message.ftm_content,
          tool_calls: message.ftm_tool_calls ?? undefined,
          thinking_blocks: message.thinking_blocks ?? undefined,
          finish_reason: message.finish_reason,
          usage: message.usage,
        },
      ]);
    }

    if (isToolMessage(message)) {
      return acc.concat([
        {
          role: "tool",
          content: message.ftm_content,
          tool_call_id: message.ftm_call_id,
        },
      ]);
    }

    if (isDiffMessage(message)) {
      const diff = {
        role: message.ftm_role,
        content: JSON.stringify(message.ftm_content),
        tool_call_id: message.tool_call_id,
      };
      return acc.concat([diff]);
    }

    const ftm_content =
      typeof message.ftm_content === "string"
        ? message.ftm_content
        : JSON.stringify(message.ftm_content);
    return [...acc, { role: message.ftm_role, content: ftm_content }];
  }, []);
}
