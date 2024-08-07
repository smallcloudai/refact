import { Snippet } from "../features/Chat2/selectedSnippet";
import {
  ChatMessages,
  ChatResponse,
  // CapsResponse,
  // isCapsResponse,
  // CommandCompletionResponse,
  ChatContextFileMessage,
  // SystemPrompts,
  // isSystemPrompts,
  ToolCommand,
  // DiffChunk,
  // DiffAppliedStateResponse,
} from "../services/refact";

export enum EVENT_NAMES_FROM_CHAT {
  SAVE_CHAT = "save_chat_to_history",
  ASK_QUESTION = "chat_question",
  // REQUEST_CAPS = "chat_request_caps",
  STOP_STREAMING = "chat_stop_streaming",
  BACK_FROM_CHAT = "chat_back_from_chat",
  OPEN_IN_CHAT_IN_TAB = "open_chat_in_new_tab",
  SEND_TO_SIDE_BAR = "chat_send_to_sidebar",
  READY = "chat_ready",
  NEW_FILE = "chat_create_new_file",
  PASTE_DIFF = "chat_paste_diff",
  // REQUEST_AT_COMMAND_COMPLETION = "chat_request_at_command_completion",
  // REQUEST_PREVIEW_FILES = "chat_request_preview_files",
  // REQUEST_PROMPTS = "chat_request_prompts",
  TAKE_NOTES = "chat_take_notes",
  // REQUEST_TOOLS = "chat_request_has_tool_check",
  // REQUEST_TOOLS = "chat_request_has_tool_check",
  // REQUEST_DIFF_APPLIED_CHUNKS = "request_diff_applied_chunks",
  // REQUEST_DIFF_OPPERATION = "request_diff_operation",
  OPEN_SETTINGS = "chat_open_settings",
  OPEN_HOT_KEYS = "chat_open_hot_keys",
}

export enum EVENT_NAMES_TO_CHAT {
  CLEAR_ERROR = "chat_clear_error",
  RESTORE_CHAT = "restore_chat_from_history",
  CHAT_RESPONSE = "chat_response",
  BACKUP_MESSAGES = "back_up_messages",
  DONE_STREAMING = "chat_done_streaming",
  ERROR_STREAMING = "chat_error_streaming",
  NEW_CHAT = "create_new_chat",
  // RECEIVE_CAPS = "receive_caps",
  // RECEIVE_CAPS_ERROR = "receive_caps_error",
  SET_CHAT_MODEL = "chat_set_chat_model",
  SET_DISABLE_CHAT = "set_disable_chat",
  ACTIVE_FILE_INFO = "chat_active_file_info",
  TOGGLE_ACTIVE_FILE = "chat_toggle_active_file",
  RECEIVE_AT_COMMAND_COMPLETION = "chat_receive_at_command_completion",
  RECEIVE_AT_COMMAND_PREVIEW = "chat_receive_at_command_preview",
  SET_SELECTED_AT_COMMAND = "chat_set_selected_command",
  SET_LAST_MODEL_USED = "chat_set_last_model_used",
  SET_SELECTED_SNIPPET = "chat_set_selected_snippet",
  REMOVE_PREVIEW_FILE_BY_NAME = "chat_remove_file_from_preview",
  SET_PREVIOUS_MESSAGES_LENGTH = "chat_set_previous_messages_length",
  RECEIVE_TOKEN_COUNT = "chat_set_tokens",
  // RECEIVE_PROMPTS = "chat_receive_prompts",
  // RECEIVE_PROMPTS_ERROR = "chat_receive_prompts_error",
  SET_SELECTED_SYSTEM_PROMPT = "chat_set_selected_system_prompt",
  SET_TAKE_NOTES = "chat_set_take_notes",
  // RECEIVE_TOOLS = "chat_receive_tools_chat",
  SET_USE_TOOLS = "chat_set_use_tools",
  SET_ENABLE_SEND = "chat_set_enable_send",
  // RECIEVE_DIFF_APPLIED_CHUNKS = "chat_recieve_diff_applied_chunks",
  // RECIEVE_DIFF_APPLIED_CHUNKS_ERROR = "chat_recieve_diff_applied_chunks_error",
  // RECIEVE_DIFF_OPPERATION_RESULT = "chat-recieve_diff_operation_result",
  // RECIEVE_DIFF_OPPERATION_ERROR = "chat-recieve_diff_operation_error",
}

export type ChatThread = {
  id: string;
  messages: ChatMessages;
  title?: string;
  model: string;
  attach_file?: boolean;
  createdAt?: string;
  lastUpdated?: string;
};

// export type Snippet = {
//   language: string;
//   code: string;
//   path: string;
//   basename: string;
// };
export interface BaseAction {
  type: EVENT_NAMES_FROM_CHAT | EVENT_NAMES_TO_CHAT;
  payload?: { id: string; [key: string]: unknown };
}

export function isBaseAction(action: unknown): action is BaseAction {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_FROM_CHAT,
    ...EVENT_NAMES_TO_CHAT,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export interface ActionFromChat extends BaseAction {
  type: EVENT_NAMES_FROM_CHAT;
}

// Will this be needed ?
export interface ReadyMessage extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.READY;
  payload: { id: string };
}

export function isReadyMessage(action: unknown): action is ReadyMessage {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.READY;
}

export function isActionFromChat(action: unknown): action is ActionFromChat {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = { ...EVENT_NAMES_FROM_CHAT };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

// export interface RequestPrompts extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_PROMPTS;
//   payload: { id: string };
// }

// export function isRequestPrompts(action: unknown): action is RequestPrompts {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_PROMPTS;
// }

// export interface RequestAtCommandCompletion extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION;
//   payload: {
//     id: string;
//     query: string;
//     cursor: number;
//     number: number;
//   };
// }

// export function isRequestAtCommandCompletion(
//   action: unknown,
// ): action is RequestAtCommandCompletion {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION;
// }

// export interface RequestPreviewFiles extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_PREVIEW_FILES;
//   payload: {
//     id: string;
//     query: string;
//   };
// }

// export function isRequestPreviewFiles(
//   action: unknown,
// ): action is RequestPreviewFiles {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_PREVIEW_FILES;
// }

export interface NewFileFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.NEW_FILE;
  payload: {
    id: string;
    content: string;
  };
}

export function isNewFileFromChat(action: unknown): action is NewFileFromChat {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.NEW_FILE;
}

export interface PasteDiffFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.PASTE_DIFF;
  payload: { id: string; content: string };
}

export function isPasteDiffFromChat(
  action: unknown,
): action is PasteDiffFromChat {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.PASTE_DIFF;
}

export interface QuestionFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION;
  payload: ChatThread & { tools: ToolCommand[] | null };
}

export function isQuestionFromChat(
  action: unknown,
): action is QuestionFromChat {
  if (!isAction(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.ASK_QUESTION;
}

export interface SaveChatFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT;
  payload: ChatThread;
}

export function isSaveChatFromChat(
  action: unknown,
): action is SaveChatFromChat {
  if (!isAction(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.SAVE_CHAT;
}

// export interface RequestCapsFromChat extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS;
//   payload: { id: string };
// }

// export function isRequestCapsFromChat(
//   action: unknown,
// ): action is RequestCapsFromChat {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_CAPS;
// }

export interface StopStreamingFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.STOP_STREAMING;
  payload: { id: string };
}

export function isStopStreamingFromChat(
  action: unknown,
): action is StopStreamingFromChat {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.STOP_STREAMING;
}

export interface TakeNotesFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.TAKE_NOTES;
  payload: ChatThread;
}

export function isTakeNotesFromChat(
  action: unknown,
): action is TakeNotesFromChat {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.TAKE_NOTES;
}

// export interface RequestTools extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_TOOLS;
//   payload: { id: string };
// }

// export function isRequestTools(action: unknown): action is RequestTools {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_TOOLS;
// }

// export interface RequestDiffAppliedChunks extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_DIFF_APPLIED_CHUNKS;
//   payload: { id: string; diff_id: string; chunks: DiffChunk[] };
// }

// export function isRequestDiffAppliedChunks(
//   action: unknown,
// ): action is RequestDiffAppliedChunks {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_DIFF_APPLIED_CHUNKS;
// }
export interface OpenSettings extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.OPEN_SETTINGS;
  payload: { id: string };
}

export function isOpenSettings(action: unknown): action is OpenSettings {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.OPEN_SETTINGS;
}

export interface OpenHotKeys extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.OPEN_HOT_KEYS;
  payload: { id: string };
}

export function isOpenHotKeys(action: unknown): action is OpenHotKeys {
  if (!isActionFromChat(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.OPEN_HOT_KEYS;
}

export interface ActionToChat extends BaseAction {
  type: EVENT_NAMES_TO_CHAT;
}

export function isActionToChat(action: unknown): action is ActionToChat {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const EVENT_NAMES: Record<string, string> = { ...EVENT_NAMES_TO_CHAT };
  return Object.values(EVENT_NAMES).includes(action.type);
}

export interface SetSelectedSystemPrompt extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SYSTEM_PROMPT;
  payload: { id: string; prompt: string };
}

export function isSetSelectedSystemPrompt(
  action: unknown,
): action is SetSelectedSystemPrompt {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_SELECTED_SYSTEM_PROMPT;
}

// export interface ReceivePrompts extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS;
//   payload: { id: string; prompts: SystemPrompts };
// }

// export function isReceivePrompts(action: unknown): action is ReceivePrompts {
//   if (!isActionToChat(action)) return false;
//   if (action.type !== EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS) return false;
//   if (!("payload" in action)) return false;
//   if (typeof action.payload !== "object") return false;
//   if (!("prompts" in action.payload)) return false;
//   return isSystemPrompts(action.payload.prompts);
// }

// export interface ReceivePromptsError extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS_ERROR;
//   payload: { id: string; error: string };
// }

// export function isReceivePromptsError(
//   action: unknown,
// ): action is ReceivePromptsError {
//   if (!isActionToChat(action)) return false;
//   if (action.type !== EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS_ERROR) return false;
//   if (!("payload" in action)) return false;
//   if (typeof action.payload !== "object") return false;
//   if (!("id" in action.payload)) return false;
//   if (typeof action.payload.id !== "string") return false;
//   if (!("error" in action.payload)) return false;
//   if (typeof action.payload.error !== "string") return false;
//   return true;
// }

// export interface ReceiveAtCommandCompletion extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_COMPLETION;
//   payload: { id: string } & CommandCompletionResponse;
// }

// export function isReceiveAtCommandCompletion(
//   action: unknown,
// ): action is ReceiveAtCommandCompletion {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_COMPLETION;
// }

export interface ReceiveAtCommandPreview extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_PREVIEW;
  payload: { id: string; preview: ChatContextFileMessage[] };
}

export function isReceiveAtCommandPreview(
  action: unknown,
): action is ReceiveAtCommandPreview {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_PREVIEW;
}

export interface SetSelectedAtCommand extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_SELECTED_AT_COMMAND;
  payload: { id: string; command: string };
}

export function isSetSelectedAtCommand(
  action: unknown,
): action is SetSelectedAtCommand {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_SELECTED_AT_COMMAND;
}

export interface ToggleActiveFile extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.TOGGLE_ACTIVE_FILE;
  payload: { id: string; attach_file: boolean };
}

export function isToggleActiveFile(
  action: unknown,
): action is ToggleActiveFile {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.TOGGLE_ACTIVE_FILE;
}

export type FileInfo = {
  name: string;
  line1: number | null;
  line2: number | null;
  can_paste: boolean;
  attach: boolean;
  path: string;
  content?: string;
  usefulness?: number;
  cursor: number | null;
};

export interface ActiveFileInfo extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.ACTIVE_FILE_INFO;
  payload: {
    id: string;
    file: Partial<FileInfo>;
  };
}

export function isActiveFileInfo(action: unknown): action is ActiveFileInfo {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.ACTIVE_FILE_INFO;
}

export interface SetChatDisable extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT;
  payload: { id: string; disable: boolean };
}

export function isSetDisableChat(action: unknown): action is SetChatDisable {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT;
}
export interface SetChatModel extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_CHAT_MODEL;
  payload: {
    id: string;
    model: string;
  };
}

export function isSetChatModel(action: unknown): action is SetChatModel {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_CHAT_MODEL;
}
export interface ResponseToChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE;
  payload: ChatResponse;
}

export function isResponseToChat(action: unknown): action is ResponseToChat {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.CHAT_RESPONSE;
}

export interface BackUpMessages extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES;
  payload: {
    id: string;
    messages: ChatMessages;
  };
}

export function isBackupMessages(action: unknown): action is BackUpMessages {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES;
}

export interface RestoreChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT;
  payload: {
    id: string;
    chat: ChatThread & {
      messages: ChatThread["messages"] | ([string, string] | null)[];
    };
    snippet?: Snippet;
  };
}

export function isRestoreChat(action: unknown): action is RestoreChat {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.RESTORE_CHAT;
}

export interface CreateNewChatThread extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.NEW_CHAT;
  payload?: { id: string; snippet?: Snippet };
}

export function isCreateNewChat(
  action: unknown,
): action is CreateNewChatThread {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.NEW_CHAT;
}

export interface ChatDoneStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.DONE_STREAMING;
  payload: { id: string };
}

export function isChatDoneStreaming(
  action: unknown,
): action is ChatDoneStreaming {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.DONE_STREAMING;
}

export interface ChatErrorStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING;
  payload: {
    id: string;
    message: string;
  };
}

export function isChatErrorStreaming(
  action: unknown,
): action is ChatErrorStreaming {
  if (!isActionToChat(action)) return false;
  if (action.type !== EVENT_NAMES_TO_CHAT.ERROR_STREAMING) return false;
  if (!("payload" in action)) return false;
  if (typeof action.payload !== "object") return false;
  if (!("id" in action.payload)) return false;
  if (typeof action.payload.id !== "string") return false;
  if (!("message" in action.payload)) return false;
  if (typeof action.payload.message !== "string") return false;
  return true;
}

export interface ChatClearError extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR;
}

export function isChatClearError(action: unknown): action is ChatClearError {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.CLEAR_ERROR;
}

// export interface ChatReceiveCaps extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS;
//   payload: {
//     id: string;
//     caps: CapsResponse;
//   };
// }

// export function isChatReceiveCaps(action: unknown): action is ChatReceiveCaps {
//   if (!isActionToChat(action)) return false;
//   if (!("payload" in action)) return false;
//   if (typeof action.payload !== "object") return false;
//   if (!("caps" in action.payload)) return false;
//   if (!isCapsResponse(action.payload.caps)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_CAPS;
// }

// export interface ChatReceiveCapsError extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS_ERROR;
//   payload: {
//     id: string;
//     message: string;
//   };
// }

// export function isChatReceiveCapsError(
//   action: unknown,
// ): action is ChatReceiveCapsError {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_CAPS_ERROR;
// }

export type Actions = ActionToChat | ActionFromChat;

export function isAction(action: unknown): action is Actions {
  return isActionFromChat(action) || isActionToChat(action);
}

export interface ChatSetLastModelUsed extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_LAST_MODEL_USED;
  payload: { id: string; model: string };
}

export function isChatSetLastModelUsed(
  action: unknown,
): action is ChatSetLastModelUsed {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_LAST_MODEL_USED;
}

// export interface ChatSetSelectedSnippet extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SNIPPET;
//   payload: { id: string; snippet: Snippet };
// }

// export function isSetSelectedSnippet(
//   action: unknown,
// ): action is ChatSetSelectedSnippet {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.SET_SELECTED_SNIPPET;
// }

export interface RemovePreviewFileByName extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.REMOVE_PREVIEW_FILE_BY_NAME;
  payload: { id: string; name: string };
}

export function isRemovePreviewFileByName(
  action: unknown,
): action is RemovePreviewFileByName {
  return (
    isActionToChat(action) &&
    action.type === EVENT_NAMES_TO_CHAT.REMOVE_PREVIEW_FILE_BY_NAME
  );
}

export interface setPreviousMessagesLength extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_PREVIOUS_MESSAGES_LENGTH;
  payload: { id: string; message_length: number };
}

export function isSetPreviousMessagesLength(
  action: unknown,
): action is setPreviousMessagesLength {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_PREVIOUS_MESSAGES_LENGTH;
}

export interface ReceiveTokenCount extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RECEIVE_TOKEN_COUNT;
  payload: { id: string; tokens: number | null };
}

export function isReceiveTokenCount(
  action: unknown,
): action is ReceiveTokenCount {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_TOKEN_COUNT;
}

export interface SetTakeNotes extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_TAKE_NOTES;
  payload: {
    id: string;
    take_notes: boolean;
  };
}

export function isSetTakeNotes(action: unknown): action is SetTakeNotes {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_TAKE_NOTES;
}

// export interface RecieveTools extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECEIVE_TOOLS;
//   payload: { id: string; tools: ToolCommand[] };
// }

// export function isRecieveTools(action: unknown): action is RecieveTools {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECEIVE_TOOLS;
// }

export interface SetUseTools extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_USE_TOOLS;
  payload: { id: string; use_tools: boolean };
}

export function isSetUseTools(action: unknown): action is SetUseTools {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_USE_TOOLS;
}

export interface SetEnableSend extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_ENABLE_SEND;
  payload: { id: string; enable_send: boolean };
}

export function isSetEnableSend(action: unknown): action is SetEnableSend {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.SET_ENABLE_SEND;
}

/**
 *   RECIEVE_DIFF_APPLIED_CHUCKS = "chat_recieve_diff_applied_chunks",
  RECIEVE_DIFF_APPLIED_CHUCKS_ERROR = "chat_recieve_diff_applied_chunks_error",
 */

// export interface RecieveDiffAppliedChunks extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS;
//   payload: {
//     id: string;
//     diff_id: string;
//     applied_chunks: boolean[];
//     can_apply: boolean[];
//   };
// }

// export function isRecieveDiffAppliedChunks(
//   action: unknown,
// ): action is RecieveDiffAppliedChunks {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS;
// }

// export interface RecieveDiffAppliedChunksError extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS_ERROR;
//   payload: { id: string; diff_id: string; reason: string };
// }

// export function isRecieveDiffAppliedChunksError(
//   action: unknown,
// ): action is RecieveDiffAppliedChunksError {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_APPLIED_CHUNKS_ERROR;
// }

// export interface RequestDiffOpperation extends ActionFromChat {
//   type: EVENT_NAMES_FROM_CHAT.REQUEST_DIFF_OPPERATION;
//   payload: {
//     id: string;
//     diff_id: string;
//     chunks: DiffChunk[];
//     toApply: boolean[];
//   };
// }

// // TODO: set fetching to true;
// export function isRequestDiffOpperation(
//   action: unknown,
// ): action is RequestDiffOpperation {
//   if (!isActionFromChat(action)) return false;
//   return action.type === EVENT_NAMES_FROM_CHAT.REQUEST_DIFF_OPPERATION;
// }

// export interface RecieveDiffOpperationResult extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_OPPERATION_RESULT;
//   payload: {
//     id: string;
//     diff_id?: string;
//     state: (0 | 1 | 2)[];
//     fuzzy_results: {
//       chunk_id: number;
//       fuzzy_n_used: number;
//     }[];
//   };
// }

// export function isRecieveDiffOpperationResult(
//   action: unknown,
// ): action is RecieveDiffOpperationResult {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_OPPERATION_RESULT;
// }

// export interface RecieveDiffOpperationError extends ActionToChat {
//   type: EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_OPPERATION_ERROR;
//   payload: { id: string; diff_id: string; reason: string };
// }

// export function isRecieveDiffOpperationError(
//   action: unknown,
// ): action is RecieveDiffOpperationError {
//   if (!isActionToChat(action)) return false;
//   return action.type === EVENT_NAMES_TO_CHAT.RECIEVE_DIFF_OPPERATION_ERROR;
// }
