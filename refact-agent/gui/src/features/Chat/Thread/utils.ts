import {
  AssistantMessage,
  ChatContextFile,
  ChatContextFileMessage,
  ChatMessage,
  ChatMessages,
  ChatResponse,
  DiffChunk,
  SubchatResponse,
  ToolCall,
  ToolMessage,
  ToolResult,
  UserMessage,
  isAssistantDelta,
  isAssistantMessage,
  isCDInstructionResponse,
  isChatContextFileDelta,
  isChatResponseChoice,
  isContextFileResponse,
  isDiffChunk,
  isDiffMessage,
  isDiffResponse,
  isLspUserMessage,
  isPlainTextResponse,
  isSubchatContextFileResponse,
  isSubchatResponse,
  isSystemResponse,
  isToolCallDelta,
  isToolContent,
  isToolMessage,
  isToolResponse,
  isUserMessage,
  isUserResponse,
} from "../../../services/refact";
import { parseOrElse } from "../../../utils";
import { type LspChatMessage } from "../../../services/refact";
import { checkForDetailMessage } from "./types";

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

function replaceLastUserMessage(
  messages: ChatMessages,
  userMessage: UserMessage,
): ChatMessages {
  if (messages.length === 0) {
    return [userMessage];
  }
  const lastUserMessageIndex = lastIndexOf<ChatMessage>(
    messages,
    isUserMessage,
  );

  const result = messages.filter((_, index) => index !== lastUserMessageIndex);

  return result.concat([userMessage]);
}

export function formatChatResponse(
  messages: ChatMessages,
  response: ChatResponse,
): ChatMessages {
  if (isUserResponse(response)) {
    return replaceLastUserMessage(messages, {
      role: response.role,
      content: response.content,
      checkpoints: response.checkpoints,
    });
  }

  if (isContextFileResponse(response)) {
    const content = parseOrElse<ChatContextFile[]>(response.content, []);
    return [...messages, { role: response.role, content }];
  }

  if (isSubchatResponse(response)) {
    return handleSubchatResponse(messages, response);
  }

  if (isToolResponse(response)) {
    const { tool_call_id, content, finish_reason } = response;
    const filteredMessages = finishToolCallInMessages(messages, tool_call_id);
    const toolResult: ToolResult =
      typeof content === "string"
        ? {
            tool_call_id,
            content,
            finish_reason,
          }
        : {
            tool_call_id,
            content,
            finish_reason,
          };

    return [...filteredMessages, { role: response.role, content: toolResult }];
  }

  if (isDiffResponse(response)) {
    const content = parseOrElse<DiffChunk[]>(response.content, []);
    return [
      ...messages,
      { role: response.role, content, tool_call_id: response.tool_call_id },
    ];
  }

  if (isPlainTextResponse(response)) {
    return [...messages, { role: response.role, content: response.content }];
  }

  if (isCDInstructionResponse(response)) {
    return [...messages, { role: response.role, content: response.content }];
  }

  // system messages go to the front
  if (isSystemResponse(response)) {
    return [{ role: response.role, content: response.content }, ...messages];
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
          finish_reason: cur.finish_reason,
        };
        return acc.concat([msg]);
      }
      // TODO: narrow this
      const message = {
        role: cur.delta.role,
        content: cur.delta.content,
        finish_reason: cur.finish_reason,
        usage: response.usage,
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
            finish_reason: cur.finish_reason,
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
        {
          role: "assistant",
          content: message,
          tool_calls: calls,
          finish_reason: cur.finish_reason,
        },
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
          finish_reason: cur.finish_reason,
        },
      ]);
    } else if (
      isAssistantDelta(cur.delta) &&
      typeof cur.delta.content === "string"
    ) {
      return acc.concat([
        {
          role: "assistant",
          content: cur.delta.content,
          finish_reason: cur.finish_reason,
        },
      ]);
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

function handleSubchatResponse(
  messages: ChatMessages,
  response: SubchatResponse,
): ChatMessages {
  function iter(
    msgs: ChatMessages,
    resp: SubchatResponse,
    accumulator: ChatMessages = [],
  ) {
    if (msgs.length === 0) return accumulator;

    const [head, ...tail] = msgs;

    if (!isAssistantMessage(head) || !head.tool_calls) {
      return iter(tail, response, accumulator.concat(head));
    }

    const maybeToolCall = head.tool_calls.find(
      (toolCall) => toolCall.id === resp.tool_call_id,
    );

    if (!maybeToolCall) return iter(tail, response, accumulator.concat(head));

    const addMessageFiles = isSubchatContextFileResponse(resp.add_message)
      ? parseOrElse<ChatContextFile[]>(resp.add_message.content, []).map(
          (file) => file.file_name,
        )
      : [];

    const attachedFiles = maybeToolCall.attached_files
      ? [...maybeToolCall.attached_files, ...addMessageFiles]
      : addMessageFiles;

    const toolCallWithCubChat: ToolCall = {
      ...maybeToolCall,
      subchat: response.subchat_id,
      attached_files: attachedFiles,
    };

    const toolCalls = head.tool_calls.map((toolCall) => {
      if (toolCall.id === toolCallWithCubChat.id) return toolCallWithCubChat;
      return toolCall;
    });

    const message: AssistantMessage = {
      ...head,
      tool_calls: toolCalls,
    };

    const nextAccumulator = [...accumulator, message];
    return iter(tail, response, nextAccumulator);
  }

  return iter(messages, response);
}

function finishToolCallInMessages(
  messages: ChatMessages,
  toolCallId: string,
): ChatMessages {
  return messages.map((message) => {
    if (!isAssistantMessage(message)) {
      return message;
    }
    if (!message.tool_calls) {
      return message;
    }
    const tool_calls = message.tool_calls.map((toolCall) => {
      if (toolCall.id !== toolCallId) {
        return toolCall;
      }
      return { ...toolCall, attached_files: undefined, subchat: undefined };
    });
    return { ...message, tool_calls };
  });
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
          finish_reason: message.finish_reason,
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

    if (
      message.role === "context_file" &&
      typeof message.content === "string"
    ) {
      const files = parseOrElse<ChatContextFile[]>(message.content, []);
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

function isValidBuffer(buffer: Uint8Array): boolean {
  // Check if the buffer is long enough
  if (buffer.length < 8) return false; // "data: " is 6 bytes + 2 bytes for "\n\n"

  // Check the start for "data: "
  const startsWithData =
    buffer[0] === 100 && // 'd'
    buffer[1] === 97 && // 'a'
    buffer[2] === 116 && // 't'
    buffer[3] === 97 && // 'a'
    buffer[4] === 58 && // ':'
    buffer[5] === 32; // ' '

  // Check the end for "\n\n"
  const endsWithNewline =
    buffer[buffer.length - 2] === 10 && // '\n'
    buffer[buffer.length - 1] === 10; // '\n'

  return startsWithData && endsWithNewline;
}

function bufferStartsWithDetail(buffer: Uint8Array): boolean {
  const startsWithDetail =
    buffer[0] === 123 && // '{'
    buffer[1] === 34 && // '"'
    buffer[2] === 100 && // 'd'
    buffer[3] === 101 && // 'e'
    buffer[4] === 116 && // 't'
    buffer[5] === 97 && // 'a'
    buffer[6] === 105 && // 'i'
    buffer[7] === 108 && // 'l'
    buffer[8] === 34 && // '"'
    buffer[9] === 58; // ':'

  return startsWithDetail;
}

export function consumeStream(
  reader: ReadableStreamDefaultReader<Uint8Array>,
  signal: AbortSignal,
  onAbort: () => void,
  onChunk: (chunk: Record<string, unknown>) => void,
) {
  const decoder = new TextDecoder();

  function pump({
    done,
    value,
  }: ReadableStreamReadResult<Uint8Array>): Promise<void> {
    if (done) return Promise.resolve();
    if (signal.aborted) {
      onAbort();
      return Promise.resolve();
    }

    if (bufferStartsWithDetail(value)) {
      const str = decoder.decode(value);
      const maybeError = checkForDetailMessage(str);
      if (maybeError) {
        const error = new Error(maybeError.detail);
        throw error;
      }
    }

    const combineBufferAndRetry = () => {
      return reader.read().then((more) => {
        if (more.done) return; // left with an invalid buffer
        const buff = new Uint8Array(value.length + more.value.length);
        buff.set(value);
        buff.set(more.value, value.length);

        return pump({ done, value: buff });
      });
    };

    if (!isValidBuffer(value)) {
      return combineBufferAndRetry();
    }

    const streamAsString = decoder.decode(value);

    const deltas = streamAsString.split("\n\n").filter((str) => str.length > 0);

    if (deltas.length === 0) return Promise.resolve();

    for (const delta of deltas) {
      if (!delta.startsWith("data: ")) {
        // eslint-disable-next-line no-console
        console.log("Unexpected data in streaming buf: " + delta);
        continue;
      }

      const maybeJsonString = delta.substring(6);

      if (maybeJsonString === "[DONE]") return Promise.resolve();

      if (maybeJsonString === "[ERROR]") {
        const errorMessage = "error from lsp";
        const error = new Error(errorMessage);

        return Promise.reject(error);
      }

      const maybeErrorData = checkForDetailMessage(maybeJsonString);
      if (maybeErrorData) {
        const errorMessage: string =
          typeof maybeErrorData.detail === "string"
            ? maybeErrorData.detail
            : JSON.stringify(maybeErrorData.detail);
        const error = new Error(errorMessage);
        // eslint-disable-next-line no-console
        console.error(error);
        throw error;
      }

      const fallback = {};
      const json = parseOrElse<Record<string, unknown>>(
        maybeJsonString,
        fallback,
      );

      if (json === fallback) {
        return combineBufferAndRetry();
      }

      onChunk(json);
    }
    return reader.read().then(pump);
  }

  return reader.read().then(pump);
}
