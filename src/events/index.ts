import { ChatMessages, ChatResponse } from "../services/refact";

export enum EVENT_NAMES_FROM_CHAT {
  SAVE_CHAT = "save_chat_to_history",
  ASK_QUESTION = "chat_question",
}

export enum EVENT_NAMES_TO_CHAT {
  CLEAR_ERROR = "chat_clear_error",
  RESTORE_CHAT = "restore_chat_from_history",
  CHAT_RESPONSE = "chat_response",
  BACKUP_MESSAGES = "back_up_messages",
  DONE_STREAMING = "chat_done_streaming",
  ERROR_STREAMING = "chat_error_streaming",
  NEW_CHAT = "create_new_chat",
}

export type ChatThread = {
  id: string;
  messages: ChatMessages;
  title?: string;
  model: string;
};
interface BaseAction {
  type: EVENT_NAMES_FROM_CHAT | EVENT_NAMES_TO_CHAT;
  payload?: { id: string };
}

export interface ActionFromChat extends BaseAction {
  type: EVENT_NAMES_FROM_CHAT;
}

export interface QuestionFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION;
  payload: ChatThread;
}

export interface SaveChatFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT;
  payload: ChatThread;
}

export interface ActionToChat extends BaseAction {
  type: EVENT_NAMES_TO_CHAT;
}
export interface ResponseToChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE;
  payload: ChatResponse;
}

export interface BackUpMessages extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES;
  payload: {
    id: string;
    messages: ChatMessages;
  };
}

export interface RestoreChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT;
  payload: ChatThread;
}

export interface CreateNewChatThread extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.NEW_CHAT;
}

export interface ChatDoneStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.DONE_STREAMING;
}

export interface ChatErrorStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING;
  payload: {
    id: string;
    message: string;
  };
}

export interface ChatClearError extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.CLEAR_ERROR;
}

export type Actions = ActionToChat | ActionFromChat;

export function isAction(action: unknown): action is Actions {
  return isActionFromChat(action) || isActionToChat(action);
}

export function isActionFromChat(action: unknown): action is ActionFromChat {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = { ...EVENT_NAMES_FROM_CHAT };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export function isQuestionFromChat(
  action: unknown,
): action is QuestionFromChat {
  if (!isAction(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.ASK_QUESTION;
}

export function isSaveChatFromChat(
  action: unknown,
): action is SaveChatFromChat {
  if (!isAction(action)) return false;
  return action.type === EVENT_NAMES_FROM_CHAT.SAVE_CHAT;
}

export function isActionToChat(action: unknown): action is ActionToChat {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const EVENT_NAMES: Record<string, string> = { ...EVENT_NAMES_TO_CHAT };
  return Object.values(EVENT_NAMES).includes(action.type);
}

export function isResponseToChat(action: unknown): action is ResponseToChat {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.CHAT_RESPONSE;
}

export function isBackupMessages(action: unknown): action is BackUpMessages {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES;
}

export function isRestoreChat(action: unknown): action is RestoreChat {
  console.log("isRestoreChat", action);
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.RESTORE_CHAT;
}

export function isCreateNewChat(
  action: unknown,
): action is CreateNewChatThread {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.NEW_CHAT;
}

export function isChatDoneStreaming(
  action: unknown,
): action is ChatDoneStreaming {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.DONE_STREAMING;
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

export function isChatClearError(action: unknown): action is ChatClearError {
  if (!isActionToChat(action)) return false;
  return action.type === EVENT_NAMES_TO_CHAT.CLEAR_ERROR;
}
