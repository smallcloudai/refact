// Careful with exports that include components, it'll cause this to compile to a large file.
import type { FileInfo } from "../features/Chat/activeFile";
// TODO: this cause more exports than needed :/
export {
  type ChatThread,
  type Chat,
  type ToolUse,
} from "../features/Chat/Thread/types";
// TODO: this may need to be re-created
// export { newChatAction } from "../features/Chat/Thread/actions";
import { type Chat } from "../features/Chat/Thread/types";
import type { Snippet } from "../features/Chat/selectedSnippet";
import type { Config } from "../features/Config/configSlice";
import type { ErrorSliceState } from "../features/Errors/errorsSlice";
import { request, ready, receive, error } from "../features/FIM/actions";
import type { TipOfTheDayState } from "../features/TipOfTheDay";
import type { PageSliceState } from "../features/Pages/pagesSlice";
import type { TourState } from "../features/Tour";
import type { FIMDebugState } from "../hooks";
import { CurrentProjectInfo } from "../features/Chat/currentProject";
import { TeamsSliceState } from "../features/Teams";

export { updateConfig, type Config } from "../features/Config/configSlice";
export { type FileInfo, setFileInfo } from "../features/Chat/activeFile";
export {
  type Snippet,
  setSelectedSnippet,
} from "../features/Chat/selectedSnippet";
export type { FimDebugData } from "../services/refact/fim";
export { addInputValue, setInputValue } from "../components/ChatForm/actions";
export {
  setCurrentProjectInfo,
  type CurrentProjectInfo,
} from "../features/Chat/currentProject";
export type { TextDocToolCall } from "../components/Tools/types";

// here
export type {
  UserMessage,
  ChatMessage,
  ChatMessages,
  DiffChunk,
  ToolEditResult,
} from "../services/refact";

import { MessagesSubscriptionSubscription } from "../../generated/documents";

export type ThreadMessage =
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread_message"];

export type {
  FThreadMultipleMessagesInput,
  FThreadMessageInput,
} from "../../generated/documents";

// TODO: re-exporting from redux seems to break things :/
export type InitialState = {
  teams: TeamsSliceState;
  fim: FIMDebugState;
  tour: TourState;
  tipOfTheDay: TipOfTheDayState;
  config: Config;
  active_file: FileInfo;
  selected_snippet: Snippet;
  chat: Chat;
  error: ErrorSliceState;
  pages: PageSliceState;
  current_project: CurrentProjectInfo;
};

export {
  ideOpenFile,
  type OpenFilePayload,
  ideDiffPasteBackAction,
  ideNewFileAction,
  ideOpenHotKeys,
  ideOpenSettingsAction,
  ideOpenChatInNewTab,
  ideAnimateFileStart,
  ideAnimateFileStop,
  ideChatPageChange,
  ideEscapeKeyPressed,
  ideIsChatStreaming,
  ideIsChatReady,
  ideToolCall,
  ideToolCallResponse,
  ideSetCodeCompletionModel,
  ideSetLoginMessage,
  ideSetActiveTeamsGroup,
  ideSetActiveTeamsWorkspace,
  ideClearActiveTeamsGroup,
  ideClearActiveTeamsWorkspace,
} from "../hooks/useEventBusForIDE";

export { ideAttachFileToChat, newChatAction } from "../hooks/useEventBusForApp";
export { toPascalCase } from "../utils/toPascalCase";
export const fim = {
  request,
  ready,
  receive,
  error,
};

export {
  isAssistantDelta,
  isAssistantMessage,
  isChatContextFileDelta,
  isChatContextFileMessage,
  isChatResponseChoice,
  isChatUserMessageResponse,
  isCommandCompletionResponse,
  isCommandPreviewResponse,
  isDetailMessage,
  isDiffMessage,
  isDiffResponse,
  isPlainTextMessage,
  isPlainTextResponse,
  isStatisticDataResponse,
  isToolCallDelta,
  isToolCallMessage,
  isToolMessage,
  isToolResponse,
  isUserMessage,
} from "../services/refact";

export type { TeamsGroup, TeamsWorkspace } from "../services/smallcloud/types";

// export type * from "../services/refact";

export * from "./setup";
export type * from "./setup";
