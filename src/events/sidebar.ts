import type { ChatHistoryItem } from "../hooks";
export type { ChatHistoryItem } from "../hooks";

// Only used for history interactions at the moment
export enum EVENT_NAMES_TO_SIDE_BAR {
  RECEIVE_CHAT_HISTORY = "sidebar_receive_chat_history",
}

export interface ActionsToSideBar {
  type: EVENT_NAMES_TO_SIDE_BAR;
}

export function isActionToSideBar(action: unknown): action is ActionsToSideBar {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_TO_SIDE_BAR,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export interface ReceiveChatHistory extends ActionsToSideBar {
  type: EVENT_NAMES_TO_SIDE_BAR.RECEIVE_CHAT_HISTORY;
  payload: ChatHistoryItem[];
}

export function isReceiveChatHistory(
  action: unknown,
): action is ReceiveChatHistory {
  if (!isActionToSideBar(action)) return false;
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
  return action.type === EVENT_NAMES_TO_SIDE_BAR.RECEIVE_CHAT_HISTORY;
}

export enum EVENT_NAMES_FROM_SIDE_BAR {
  READY = "sidebar_ready",
  OPEN_CHAT_IN_SIDEBAR = "sidebar_open_chat_in_sidebar",
  OPEN_IN_CHAT_IN_TAB = "sidebar_open_chat_in_tab",
  DELETE_HISTORY_ITEM = "sidebar_delete_history_item",
  REQUEST_CHAT_HISTORY = "sidebar_request_chat_history",
  CREATE_NEW_CHAT = "sidebar_create_new_chat",
}

export interface ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR;
}

export function isActionFromSidebar(
  action: unknown,
): action is ActionFromSidebar {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_FROM_SIDE_BAR,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export interface SidebarReady extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.READY;
}

export function isSidebarReady(action: unknown): action is SidebarReady {
  if (!isActionFromSidebar(action)) return false;
  return action.type === EVENT_NAMES_FROM_SIDE_BAR.READY;
}

export interface OpenChatInSidebar extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.OPEN_CHAT_IN_SIDEBAR;
  payload: { id: string };
}

export function isOpenChatInSidebar(
  action: unknown,
): action is OpenChatInSidebar {
  return (
    isActionFromSidebar(action) &&
    action.type === EVENT_NAMES_FROM_SIDE_BAR.OPEN_CHAT_IN_SIDEBAR
  );
}

export interface OpenChatInTab extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.OPEN_IN_CHAT_IN_TAB;
  payload: { id: string };
}

export function isOpenChatInTab(action: unknown): action is OpenChatInTab {
  if (!isActionFromSidebar(action)) return false;
  return action.type === EVENT_NAMES_FROM_SIDE_BAR.OPEN_IN_CHAT_IN_TAB;
}

export interface DeleteHistoryItem extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.DELETE_HISTORY_ITEM;
  payload: { id: string };
}

export function isDeleteChatHistory(
  action: unknown,
): action is DeleteHistoryItem {
  if (!isActionFromSidebar(action)) return false;
  return action.type === EVENT_NAMES_FROM_SIDE_BAR.DELETE_HISTORY_ITEM;
}

export interface RequestChatHistory extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.REQUEST_CHAT_HISTORY;
}

export function isRequestChatHistory(
  action: unknown,
): action is RequestChatHistory {
  if (!isActionFromSidebar(action)) return false;
  return action.type === EVENT_NAMES_FROM_SIDE_BAR.REQUEST_CHAT_HISTORY;
}

export interface CreateNewChatFromSidebar extends ActionFromSidebar {
  type: EVENT_NAMES_FROM_SIDE_BAR.CREATE_NEW_CHAT;
}

export function isCreateNewChatFromSidebar(
  action: unknown,
): action is CreateNewChatFromSidebar {
  if (!isActionFromSidebar(action)) return false;
  return action.type === EVENT_NAMES_FROM_SIDE_BAR.CREATE_NEW_CHAT;
}
