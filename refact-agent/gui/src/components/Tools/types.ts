import { ToolCall } from "../../services/refact";
import { parseOrElse } from "../../utils";

export const TEXTDOC_TOOL_NAMES = [
  "create_textdoc",
  "update_textdoc",
  "replace_textdoc",
  "update_textdoc_regex",
  "update_textdoc_by_lines",
];

type TextDocToolNames = (typeof TEXTDOC_TOOL_NAMES)[number];

export interface RawTextDocTool extends ToolCall {
  function: {
    name: TextDocToolNames;
    arguments: string; // stringified json
  };
}

// To use when deciding to render TextDoc
export const isRawTextDocToolCall = (
  toolCall: ToolCall,
): toolCall is RawTextDocTool => {
  if (!toolCall.function.name) return false;
  if (typeof toolCall.function.arguments !== "string") return false;
  return TEXTDOC_TOOL_NAMES.includes(toolCall.function.name);
};

export type ParsedRawTextDocToolCall = Omit<RawTextDocTool, "function"> & {
  function: {
    name: TextDocToolNames;
    arguments: Record<string, string | boolean>;
  };
};

export const isParseRawTextDocToolCall = (
  json: unknown,
): json is ParsedRawTextDocToolCall => {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("function" in json)) return false;
  if (!json.function) return false;
  if (typeof json.function !== "object") return false;
  if (!("name" in json.function)) return false;
  if (!json.function.name) return false;
  if (!("arguments" in json.function)) return false;
  if (!json.function.arguments) return false;
  if (typeof json.function.arguments !== "object") return false;
  return true;
};

export interface CreateTextDocToolCall extends ParsedRawTextDocToolCall {
  function: {
    name: "create_textdoc";
    arguments: {
      path: string;
      content: string;
    };
  };
}

export const isCreateTextDocToolCall = (
  toolCall: ParsedRawTextDocToolCall,
): toolCall is CreateTextDocToolCall => {
  if (toolCall.function.name !== "create_textdoc") return false;
  if (!("path" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.path !== "string") return false;
  if (!("content" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.content !== "string") return false;
  return true;
};

export interface UpdateTextDocToolCall extends ParsedRawTextDocToolCall {
  function: {
    name: "update_textdoc";
    arguments: {
      path: string;
      old_str: string;
      replacement: string;
      multiple: boolean;
    };
  };
}

export const isUpdateTextDocToolCall = (
  toolCall: ParsedRawTextDocToolCall,
): toolCall is UpdateTextDocToolCall => {
  if (toolCall.function.name !== "update_textdoc") return false;
  if (!("path" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.path !== "string") return false;
  if (!("old_str" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.old_str !== "string") return false;
  if (!("replacement" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.replacement !== "string") return false;
  if (
    "multiple" in toolCall.function.arguments &&
    typeof toolCall.function.arguments.multiple !== "boolean"
  )
    return false;
  return true;
};

export interface UpdateRegexTextDocToolCall extends ParsedRawTextDocToolCall {
  function: {
    name: string;
    arguments: {
      path: string;
      pattern: string;
      replacement: string;
      multiple: boolean;
    };
  };
}

export const isUpdateRegexTextDocToolCall = (
  toolCall: ParsedRawTextDocToolCall,
): toolCall is UpdateRegexTextDocToolCall => {
  if (toolCall.function.name !== "update_textdoc_regex") return false;
  if (!("path" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.path !== "string") return false;
  if (!("pattern" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.pattern !== "string") return false;
  if (!("replacement" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.replacement !== "string") return false;
  if (
    "multiple" in toolCall.function.arguments &&
    typeof toolCall.function.arguments.multiple !== "boolean"
  )
    return false;
  return true;
};

export interface ReplaceTextDocToolCall extends ParsedRawTextDocToolCall {
  function: {
    name: string;
    arguments: {
      path: string;
      replacement: string;
    };
  };
}

export const isReplaceTextDocToolCall = (
  toolCall: ParsedRawTextDocToolCall,
): toolCall is ReplaceTextDocToolCall => {
  if (toolCall.function.name !== "replace_textdoc") return false;
  if (!("path" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.path !== "string") return false;
  if (!("replacement" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.replacement !== "string") return false;
  return true;
};

export interface UpdateTextDocByLinesToolCall extends ParsedRawTextDocToolCall {
  function: {
    name: string;
    arguments: {
      path: string;
      content: string;
      ranges: string;
    };
  };
}

export const isUpdateTextDocByLinesToolCall = (
  toolCall: ParsedRawTextDocToolCall,
): toolCall is UpdateTextDocByLinesToolCall => {
  if (toolCall.function.name !== "update_textdoc_by_lines") return false;
  if (!("path" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.path !== "string") return false;
  if (!("content" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.content !== "string") return false;
  if (!("ranges" in toolCall.function.arguments)) return false;
  if (typeof toolCall.function.arguments.ranges !== "string") return false;
  return true;
};

export type TextDocToolCall =
  | CreateTextDocToolCall
  | UpdateTextDocToolCall
  | ReplaceTextDocToolCall
  | UpdateRegexTextDocToolCall
  | UpdateTextDocByLinesToolCall;

function isTextDocToolCall(
  toolCall: ParsedRawTextDocToolCall,
): toolCall is TextDocToolCall {
  if (isCreateTextDocToolCall(toolCall)) return true;
  if (isUpdateTextDocToolCall(toolCall)) return true;
  if (isReplaceTextDocToolCall(toolCall)) return true;
  if (isUpdateRegexTextDocToolCall(toolCall)) return true;
  if (isUpdateTextDocByLinesToolCall(toolCall)) return true;
  return false;
}

export function parseRawTextDocToolCall(
  toolCall: RawTextDocTool,
): TextDocToolCall | null {
  const parsedArguments = parseOrElse<Record<string, string | boolean>>(
    toolCall.function.arguments,
    {},
  );
  const parsedToolCallWithArgs = {
    ...toolCall,
    function: { ...toolCall.function, arguments: parsedArguments },
  };

  if (!isParseRawTextDocToolCall(parsedToolCallWithArgs)) return null;

  if (!isTextDocToolCall(parsedToolCallWithArgs)) return null;

  return parsedToolCallWithArgs;
}
