import {
  // AssistantMessage,
  ChatContextFile,
  ChatContextFileMessage,
  // ChatMessage,
  ChatMessages,
  // ChatResponse,
  // DiffChunk,
  // SubchatResponse,
  ToolCall,
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
  ThinkingBlock,
  // isToolCallMessage,
  // Usage,
  LSPUserMessage,
} from "../../../services/refact";
import { v4 as uuidv4 } from "uuid";
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

// TODO: can remove

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
      const files = parseOrElse<ChatContextFile[]>(message.content, []);
      const contextFileMessage: ChatContextFileMessage = {
        ftm_role: message.role,
        ftm_content: files,
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
        return Promise.reject(maybeError);
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

      if (maybeJsonString === "[DONE]") {
        return Promise.resolve();
      }

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
        return Promise.reject(maybeErrorData);
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
