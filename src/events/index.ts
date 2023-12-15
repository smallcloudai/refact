import { ChatMessages, ChatResponse } from "../services/refact";

export enum EVENT_NAMES_FROM_CHAT {
    SAVE_CHAT = "save_chat_to_history",
    ASK_QUESTION = "chat_question",
}

export enum EVENT_NAMES_TO_CHAT {
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
    type: string;
    payload?: unknown;
}

export interface MessageFromChat extends BaseAction {
    type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE;
    payload: ChatResponse;
  }

export interface BackUpMessages extends BaseAction {
    type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES;
    payload: ChatMessages;
}

export interface RestoreChat extends BaseAction {
    type:  EVENT_NAMES_TO_CHAT.RESTORE_CHAT
    payload: ChatThread;
}

export interface NewChatThread extends BaseAction {
    type: EVENT_NAMES_TO_CHAT.NEW_CHAT;
}

export type Actions = MessageFromChat | BackUpMessages | RestoreChat | NewChatThread;