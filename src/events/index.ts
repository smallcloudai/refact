// Careful with exports that include components, it'll cause this to compile to a large file.
import { request, ready, receive, error } from "../features/FIM";
export { type FileInfo, setFileInfo } from "../features/Chat/activeFile";
export {
  type Snippet,
  setSelectedSnippet,
} from "../features/Chat/selectedSnippet";

export type { FimDebugData } from "../services/refact/fim";

export {
  ideOpenFile,
  type OpenFilePayload,
  ideDiffPasteBackAction,
  ideNewFileAction,
  ideOpenHotKeys,
  ideOpenSettingsAction,
} from "../hooks/useEventBusForIDE";

export const fim = {
  request,
  ready,
  receive,
  error,
};

export {
  isAssistantDelta,
  isAssistantMessage,
  isCapsResponse,
  isChatContextFileDelta,
  isChatContextFileMessage,
  isChatResponseChoice,
  isChatUserMessageResponse,
  isCommandCompletionResponse,
  isCommandPreviewResponse,
  isCustomPromptsResponse,
  isDetailMessage,
  isDiffMessage,
  isDiffResponse,
  isPlainTextMessage,
  isPlainTextResponse,
  isStatisticDataResponse,
  isSystemPrompts,
  isToolCallDelta,
  isToolCallMessage,
  isToolMessage,
  isToolResponse,
  isUserMessage,
} from "../services/refact";

export type * from "../services/refact";

export * from "./config";
export type * from "./config";

export * from "./setup";
export type * from "./setup";
