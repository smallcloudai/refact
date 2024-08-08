import {
  AssistantMessage,
  ChatContextFile,
  ChatMessage,
  ChatMessages,
  ChatResponse,
  ContextMemory,
  DiffChunk,
  ToolCall,
  ToolResult,
  isAssistantDelta,
  isAssistantMessage,
  isChatContextFileDelta,
  isChatResponseChoice,
  isChatUserMessageResponse,
  isDiffMessage,
  isDiffResponse,
  isPlainTextResponse,
  isToolCallDelta,
  isToolMessage,
  isToolResponse,
} from "../../services/refact";
import { parseOrElse } from "../../utils";
import { type LspChatMessage } from "../../services/refact";

// export const TAKE_NOTE_MESSAGE = [
//   'How many times user has corrected or directed you? Write "Number of correction points N".',
//   'Then start each one with "---\n", describe what you (the assistant) did wrong, write "Mistake: ..."',
//   'Write documentation to tools or the project in general that will help you next time, describe in detail how tools work, or what the project consists of, write "Documentation: ..."',
//   "A good documentation for a tool describes what is it for, how it helps to answer user's question, what applicability criteia were discovered, what parameters work and how it will help the user.",
//   "A good documentation for a project describes what folders, files are there, summarization of each file, classes. Start documentation for the project with project name.",
//   "After describing all points, call note_to_self() in parallel for each actionable point, generate keywords that should include the relevant tools, specific files, dirs, and put documentation-like paragraphs into text.",
// ].join("\n");

// export const TAKE_NOTE_MESSAGE = [
//   "How many times user has corrected you about tool usage? Call note_to_self() with this exact format:",
//   "",
//   "CORRECTION_POINTS: N",
//   "",
//   "POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan",
//   "POINT1 WAS_I_SUCCESSFUL_AFTER_CORRECTION: YES/NO",
//   "POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan",
//   "POINT1 HOW_NEW_IS_THIS_NOTE: 0-5",
//   "POINT1 HOW_INSIGHTFUL_IS_THIS_NOTE: 0-5",
//   "",
//   "POINT2 WHAT_I_DID_WRONG: ...",
//   "POINT2 WAS_I_SUCCESSFUL_AFTER_CORRECTION: ...",
//   "POINT2 FOR_FUTURE_FEREFENCE: ...",
//   "POINT2 HOW_NEW_IS_THIS_NOTE: ...",
//   "POINT2 HOW_INSIGHTFUL_IS_THIS_NOTE: ...",
// ].join("\n");

export const TAKE_NOTE_MESSAGE = `How many times did you used a tool incorrectly, so it didn't produce the indended result? Call remember_how_to_use_tools() with this exact format:

CORRECTION_POINTS: N

POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan.
POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan.

POINT2 WHAT_I_DID_WRONG: ...
POINT2 FOR_FUTURE_FEREFENCE: ...
`;

function mergeToolCall(prev: ToolCall[], add: ToolCall): ToolCall[] {
  const calls = prev.slice();

  if (calls[add.index]) {
    const prevCall = calls[add.index];
    const prevArgs = prevCall.function.arguments;
    const nextArgs = prevArgs + add.function.arguments;
    const call: ToolCall = {
      ...prevCall,
      function: {
        ...prevCall.function,
        arguments: nextArgs,
      },
    };
    calls[add.index] = call;
  } else {
    calls[add.index] = add;
  }
  return calls;
}

export function mergeToolCalls(prev: ToolCall[], add: ToolCall[]): ToolCall[] {
  return add.reduce((acc, cur) => {
    return mergeToolCall(acc, cur);
  }, prev);
}

export function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  if (isChatUserMessageResponse(response)) {
    if (response.role === "context_file") {
      const content = parseOrElse<ChatContextFile[]>(response.content, []);
      // const msg: ChatContextFileMessage = { role: response.role, content };
      return [...messages, { role: response.role, content }];
    } else if (response.role === "context_memory") {
      const content = parseOrElse<ContextMemory[]>(response.content, []);
      return [...messages, { role: response.role, content }];
    }

    return [...messages, { role: response.role, content: response.content }];
  }

  if (isToolResponse(response)) {
    const { tool_call_id, content, finish_reason } = response;
    const toolResult: ToolResult = { tool_call_id, content, finish_reason };
    return [...messages, { role: response.role, content: toolResult }];
  }

  if (isDiffResponse(response)) {
    const content = parseOrElse<DiffChunk[]>(response.content, []);
    return [
      ...messages,
      { role: response.role, content, tool_call_id: response.tool_call_id },
    ];
  }

  if (isPlainTextResponse(response)) {
    return [...messages, response];
  }

  if (!isChatResponseChoice(response)) {
    // console.log("Not a good response");
    // console.log(response);
    return messages;
  }

  return response.choices.reduce<ChatMessages>((acc, cur) => {
    if (isChatContextFileDelta(cur.delta)) {
      const msg = { role: cur.delta.role, content: cur.delta.content };
      return acc.concat([msg]);
    }

    if (
      acc.length === 0 &&
      "content" in cur.delta &&
      typeof cur.delta.content === "string" &&
      cur.delta.role
    ) {
      if (cur.delta.role === "assistant") {
        const msg: AssistantMessage = {
          role: cur.delta.role,
          content: cur.delta.content,
          tool_calls: cur.delta.tool_calls,
        };
        return acc.concat([msg]);
      }
      // TODO: narrow this
      const message = {
        role: cur.delta.role,
        content: cur.delta.content,
      } as ChatMessage;
      return acc.concat([message]);
    }

    const lastMessage = acc[acc.length - 1];

    if (isToolCallDelta(cur.delta)) {
      if (!isAssistantMessage(lastMessage)) {
        return acc.concat([
          {
            role: "assistant",
            content: cur.delta.content ?? "",
            tool_calls: cur.delta.tool_calls,
          },
        ]);
      }

      const last = acc.slice(0, -1);
      const collectedCalls = lastMessage.tool_calls ?? [];
      const calls = mergeToolCalls(collectedCalls, cur.delta.tool_calls);
      const content = cur.delta.content;
      const message = content
        ? lastMessage.content + content
        : lastMessage.content;

      return last.concat([
        { role: "assistant", content: message, tool_calls: calls },
      ]);
    }

    if (
      isAssistantMessage(lastMessage) &&
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      const last = acc.slice(0, -1);
      const currentMessage = lastMessage.content ?? "";
      const toolCalls = lastMessage.tool_calls;
      return last.concat([
        {
          role: "assistant",
          content: currentMessage + cur.delta.content,
          tool_calls: toolCalls,
        },
      ]);
    } else if (
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      return acc.concat([{ role: "assistant", content: cur.delta.content }]);
    } else if (cur.delta.role === "assistant") {
      // empty message from JB
      return acc;
    }

    if (cur.delta.role === null || cur.finish_reason !== null) {
      return acc;
    }

    // console.log("Fall though");
    // console.log({ cur, lastMessage });

    return acc;
  }, messages);
}

export function formatMessagesForLsp(messages: ChatMessages): LspChatMessage[] {
  return messages.reduce<LspChatMessage[]>((acc, message) => {
    if (isAssistantMessage(message)) {
      return acc.concat([
        {
          role: message.role,
          content: message.content,
          tool_calls: message.tool_calls ?? undefined,
        },
      ]);
    }

    if (isToolMessage(message)) {
      return acc.concat([
        {
          role: "tool",
          content: message.content.content,
          tool_call_id: message.content.tool_call_id,
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
