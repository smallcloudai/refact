// Careful with exports that include components, it'll cause this to compile to a large file.
import { FileInfo } from "../features/Chat/activeFile";
import { Chat } from "../features/Chat/chatThread";
import { Snippet } from "../features/Chat/selectedSnippet";
import { Config } from "../features/Config/configSlice";
import { ErrorSliceState } from "../features/Errors/errorsSlice";
import { request, ready, receive, error } from "../features/FIM";
import { HistoryState } from "../features/History/historySlice";
import { TipOfTheDayState } from "../features/TipOfTheDay";
import { TourState } from "../features/Tour";
import { FIMDebugState } from "../hooks";
// import { rootReducer } from "../app/store";
export { updateConfig, type Config } from "../features/Config/configSlice";
export { type FileInfo, setFileInfo } from "../features/Chat/activeFile";
export {
  type Snippet,
  setSelectedSnippet,
} from "../features/Chat/selectedSnippet";
export type { FimDebugData } from "../services/refact/fim";

// TODO: re-exporting from redux seems to break things :/
export type InitialState = {
  fim: FIMDebugState;
  tour: TourState;
  tipOfTheDay: TipOfTheDayState;
  config: Config;
  active_file: FileInfo;
  selected_snippet: Snippet;
  chat: Chat;
  history: HistoryState;
  error: ErrorSliceState;
};

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

export * from "./setup";
export type * from "./setup";
