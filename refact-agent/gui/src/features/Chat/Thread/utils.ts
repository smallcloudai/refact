import {
  // AssistantMessage,
  // ChatContextFile,
  ChatContextFileMessage,
  // ChatMessage,
  ChatMessages,
  // ChatResponse,
  // DiffChunk,
  // SubchatResponse,
  ToolMessage,
  UserMessage,
  // isAssistantDelta,
  isAssistantMessage,
  // isCDInstructionResponse,
  // isChatContextFileDelta,
  // isChatResponseChoice,
  // isContextFileResponse,
  isDiffChunk,
  isDiffMessage,
  // isDiffResponse,
  isLspUserMessage,
  // isPlainTextResponse,
  // isSubchatContextFileResponse,
  // isSubchatResponse,
  // isSystemResponse,
  // isToolCallDelta,
  // isThinkingBlocksDelta,
  isToolContent,
  isToolMessage,
  // isToolResponse,
  isUserMessage,
  // isUserResponse,
  // ThinkingBlock,
  // isToolCallMessage,
  // Usage,
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

export function formatMessagesForChat(
  messages: LspChatMessage[],
): ChatMessages {
  return messages.reduce<ChatMessages>((acc, message) => {
    if (isLspUserMessage(message) && typeof message.content === "string") {
      const userMessage: UserMessage = {
        ftm_role: message.role,
        ftm_content: message.content,
        checkpoints: message.checkpoints,
      };
      return acc.concat(userMessage);
    }

    if (message.role === "assistant") {
      const { role, content, ...rest } = message;
      return acc.concat({
        ftm_role: role,
        ftm_content: content,
        ...rest,
      });
    }

    if (
      message.role === "context_file" &&
      typeof message.content === "string"
    ) {
      const contextFileMessage: ChatContextFileMessage = {
        ftm_role: message.role,
        ftm_content: message.content,
      };
      return acc.concat(contextFileMessage);
    }

    if (message.role === "system" && typeof message.content === "string") {
      return acc.concat({
        ftm_role: message.role,
        ftm_content: message.content,
      });
    }

    if (message.role === "plain_text" && typeof message.content === "string") {
      return acc.concat({
        ftm_role: message.role,
        ftm_content: message.content,
      });
    }

    if (
      message.role === "cd_instruction" &&
      typeof message.content === "string"
    ) {
      return acc.concat({
        ftm_role: message.role,
        ftm_content: message.content,
      });
    }

    if (
      message.role === "tool" &&
      (typeof message.content === "string" || isToolContent(message.content)) &&
      typeof message.tool_call_id === "string"
    ) {
      // TODO: why type cast this
      return acc.concat(message as unknown as ToolMessage);
    }

    if (
      message.role === "diff" &&
      Array.isArray(message.content) &&
      message.content.every(isDiffChunk) &&
      typeof message.tool_call_id === "string"
    ) {
      return acc.concat({
        ftm_role: message.role,
        ftm_content: message.content,
        tool_call_id: message.tool_call_id,
      });
    }

    return acc;
  }, []);
}
