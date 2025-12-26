import {
  AssistantMessage,
  ChatContextFile,
  ChatContextFileMessage,
  ChatMessages,
  ToolCall,
  ToolMessage,
  UserMessage,
  isAssistantMessage,
  isDiffChunk,
  isDiffMessage,
  isLspUserMessage,
  isToolContent,
  isToolMessage,
  isUserMessage,
  ThinkingBlock,
} from "../../../services/refact";
import { v4 as uuidv4 } from "uuid";
import { parseOrElse } from "../../../utils";
import { type LspChatMessage } from "../../../services/refact";
import { isServerExecutedTool } from "./types";

export function postProcessMessagesAfterStreaming(
  messages: ChatMessages,
): ChatMessages {
  return messages.map((message) => {
    if (!isAssistantMessage(message) || !message.tool_calls) {
      return message;
    }

    const deduplicatedTools = deduplicateToolCalls(message.tool_calls);
    const ignoredTools: ToolCall[] = [];
    const keptTools: ToolCall[] = [];

    deduplicatedTools.forEach((tool) => {
      // Server-executed tools (srvtoolu_*) are already executed by the LLM provider
      // They should not be sent to backend for execution
      if (isServerExecutedTool(tool.id)) {
        ignoredTools.push(tool);
      } else {
        keptTools.push(tool);
      }
    });

    if (
      ignoredTools.length === 0 &&
      deduplicatedTools.length === message.tool_calls.length
    ) {
      return message;
    }

    // Store server-executed tools for display, but don't send them to backend
    return {
      ...message,
      tool_calls: keptTools.length > 0 ? keptTools : undefined,
      server_executed_tools: ignoredTools.length > 0 ? ignoredTools : undefined,
    };
  });
}

function deduplicateToolCalls(toolCalls: ToolCall[]): ToolCall[] {
  const toolCallMap = new Map<string, ToolCall>();

  toolCalls.forEach((tool) => {
    if (!tool.id) return; // Skip tools without an id
    const existingTool = toolCallMap.get(tool.id);

    if (!existingTool) {
      toolCallMap.set(tool.id, tool);
    } else {
      const existingHasArgs =
        existingTool.function.arguments &&
        existingTool.function.arguments.trim() !== "";
      const newHasArgs =
        tool.function.arguments && tool.function.arguments.trim() !== "";

      if (!existingHasArgs && newHasArgs) {
        toolCallMap.set(tool.id, tool);
      }
    }
  });

  return Array.from(toolCallMap.values());
}

export const TAKE_NOTE_MESSAGE = `How many times did you used a tool incorrectly, so it didn't produce the indented result? Call remember_how_to_use_tools() with this exact format:

CORRECTION_POINTS: N

POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan.
POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan.

POINT2 WHAT_I_DID_WRONG: ...
POINT2 FOR_FUTURE_FEREFENCE: ...
`;

function mergeToolCall(prev: ToolCall[], add: ToolCall): ToolCall[] {
  const calls = prev.slice();

  // NOTE: we can't be sure that backend sends correct indexes for tool calls
  // in case of qwen3 with sglang I get 2 problems fixed here:
  // 1. index of first tool call delta == 2 next == 0 (huh?)
  // 2. second tool call in a row has id == null
  if (!calls.length || add.function.name) {
    add.index = calls.length;
    if (!add.id) {
      add.id = uuidv4();
    }
    calls[calls.length] = add;
  } else {
    const prevCall = calls[calls.length - 1];
    const prevArgs = prevCall.function.arguments;
    const nextArgs = prevArgs + add.function.arguments;
    const call: ToolCall = {
      ...prevCall,
      function: {
        ...prevCall.function,
        arguments: nextArgs,
      },
    };
    calls[calls.length - 1] = call;
  }
  return calls;
}

export function mergeToolCalls(prev: ToolCall[], add: ToolCall[]): ToolCall[] {
  return add.reduce((acc, cur) => {
    return mergeToolCall(acc, cur);
  }, prev);
}

function mergeThinkingBlock(
  prev: ThinkingBlock[],
  add: ThinkingBlock,
): ThinkingBlock[] {
  if (prev.length === 0) {
    return [add];
  } else {
    return [
      {
        ...prev[0],
        thinking: (prev[0].thinking ?? "") + (add.thinking ?? ""),
        signature: (prev[0].signature ?? "") + (add.signature ?? ""),
      },
      ...prev.slice(1),
    ];
  }
}

export function mergeThinkingBlocks(
  prev: ThinkingBlock[],
  add: ThinkingBlock[],
): ThinkingBlock[] {
  return add.reduce((acc, cur) => {
    return mergeThinkingBlock(acc, cur);
  }, prev);
}

export function lastIndexOf<T>(arr: T[], predicate: (a: T) => boolean): number {
  let index = -1;
  for (let i = arr.length - 1; i >= 0; i--) {
    if (predicate(arr[i])) {
      index = i;
      break;
    }
  }
  return index;
}



export function formatMessagesForLsp(messages: ChatMessages): LspChatMessage[] {
  return messages.reduce<LspChatMessage[]>((acc, message) => {
    if (isUserMessage(message)) {
      return acc.concat([message]);
    }

    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message.role,
          content: message.content,
          tool_calls: message.tool_calls ?? undefined,
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
          content: message.content,
          tool_call_id: message.tool_call_id,
        },
      ]);
    }

    if (isDiffMessage(message)) {
      const diff = {
        role: message.role,
        content: JSON.stringify(message.content),
        tool_call_id: message.tool_call_id,
      };
      return acc.concat([diff]);
    }

    const content =
      typeof message.content === "string"
        ? message.content
        : JSON.stringify(message.content);
    return [...acc, { role: message.role, content }];
  }, []);
}

export function formatMessagesForChat(
  messages: LspChatMessage[],
): ChatMessages {
  return messages.reduce<ChatMessages>((acc, message) => {
    if (isLspUserMessage(message) && typeof message.content === "string") {
      const userMessage: UserMessage = {
        role: message.role,
        content: message.content,
        checkpoints: message.checkpoints,
      };
      return acc.concat(userMessage);
    }

    if (message.role === "assistant") {
      // TODO: why type cast this.
      const assistantMessage = message as AssistantMessage;
      return acc.concat({
        ...assistantMessage,
      });
    }

    if (message.role === "context_file") {
      let files: ChatContextFile[];
      if (typeof message.content === "string") {
        files = parseOrElse<ChatContextFile[]>(message.content, []);
      } else if (Array.isArray(message.content)) {
        files = message.content as ChatContextFile[];
      } else {
        files = [];
      }
      const contextFileMessage: ChatContextFileMessage = {
        role: message.role,
        content: files,
      };
      return acc.concat(contextFileMessage);
    }

    if (message.role === "system" && typeof message.content === "string") {
      return acc.concat({ role: message.role, content: message.content });
    }

    if (message.role === "plain_text" && typeof message.content === "string") {
      return acc.concat({ role: message.role, content: message.content });
    }

    if (
      message.role === "cd_instruction" &&
      typeof message.content === "string"
    ) {
      return acc.concat({ role: message.role, content: message.content });
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
        role: message.role,
        content: message.content,
        tool_call_id: message.tool_call_id,
      });
    }

    return acc;
  }, []);
}
