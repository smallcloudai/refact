import { request, ready, receive, error } from "../features/FIM";
export {
  type FileInfo,
  setFileInfo,
  type Snippet,
  setSelectedSnippet,
} from "../features/Chat";

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

export * from "../services/refact";
export type * from "../services/refact";
export * from "./config";
export type * from "./config";

// TODO: Export events for vscode
export * from "./setup";
export type * from "./setup";
