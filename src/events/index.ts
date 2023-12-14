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