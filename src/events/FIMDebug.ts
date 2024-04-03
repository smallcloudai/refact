import type { FimDebugData } from "../services/refact";

// Maybe sub type these?
export enum EVENT_NAMES_FROM_FIM_DEBUG {
  READY = "fim_debug_ready",
  REQUEST_FIM_DEBUG_DATA = "request_fim_debug_data",
}

interface ActionFromFIMDebug {
  type: EVENT_NAMES_FROM_FIM_DEBUG;
}

const ALL_EVENT_NAMES_FROM_FIM_DEBUG: string[] = Object.values(
  EVENT_NAMES_FROM_FIM_DEBUG,
);

export function isActionFromFIMDebug(
  action: unknown,
): action is ActionFromFIMDebug {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  return ALL_EVENT_NAMES_FROM_FIM_DEBUG.includes(action.type);
}

export interface FIMDebugReady extends ActionFromFIMDebug {
  type: EVENT_NAMES_FROM_FIM_DEBUG.READY;
}

export function isReadyMessageFromFIMDebug(
  action: unknown,
): action is FIMDebugReady {
  if (!isActionFromFIMDebug(action)) return false;
  return action.type === EVENT_NAMES_FROM_FIM_DEBUG.READY;
}

export enum EVENT_NAMES_TO_FIM_DEBUG {
  RECEIVE_FIM_DEBUG_DATA = "receive_fim_debug_data",
  RECEIVE_FIM_DEBUG_ERROR = "receive_fim_debug_error",
  CLEAR_ERROR = "fim_debug_clear_error",
}

export interface ActionToFIMDebug {
  type: EVENT_NAMES_TO_FIM_DEBUG;
}

const ALL_EVENT_NAMES_TO_FIM_DEBUG: string[] = Object.values(
  EVENT_NAMES_TO_FIM_DEBUG,
);

export function isActionToFIMDebug(
  action: unknown,
): action is ActionToFIMDebug {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  return ALL_EVENT_NAMES_TO_FIM_DEBUG.includes(action.type);
}

export interface ClearFIMDebugError extends ActionToFIMDebug {
  type: EVENT_NAMES_TO_FIM_DEBUG.CLEAR_ERROR;
}

export function isClearFIMDebugError(
  action: unknown,
): action is ClearFIMDebugError {
  if (!isActionToFIMDebug(action)) return false;
  return action.type === EVENT_NAMES_TO_FIM_DEBUG.CLEAR_ERROR;
}

export interface ReceiveFIMDebugData extends ActionToFIMDebug {
  type: EVENT_NAMES_TO_FIM_DEBUG.RECEIVE_FIM_DEBUG_DATA;
  payload: FimDebugData;
}

export function isReceiveFIMDebugData(
  action: unknown,
): action is ReceiveFIMDebugData {
  if (!isActionToFIMDebug(action)) return false;
  return action.type === EVENT_NAMES_TO_FIM_DEBUG.RECEIVE_FIM_DEBUG_DATA;
}

export interface ReceiveFIMDebugError extends ActionToFIMDebug {
  type: EVENT_NAMES_TO_FIM_DEBUG.RECEIVE_FIM_DEBUG_ERROR;
  payload: {
    message: string;
  };
}

export function isReceiveFIMDebugError(
  action: unknown,
): action is ReceiveFIMDebugError {
  if (!isActionToFIMDebug(action)) return false;
  if (action.type !== EVENT_NAMES_TO_FIM_DEBUG.RECEIVE_FIM_DEBUG_ERROR)
    return false;
  if (!("payload" in action)) return false;
  if (typeof action.payload !== "object") return false;
  if (action.payload === null) return false;
  if (!("message" in action.payload)) return false;
  return typeof action.payload.message === "string";
}
