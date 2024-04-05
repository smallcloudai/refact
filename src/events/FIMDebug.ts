import type { FimDebugData } from "../services/refact";

export enum FIM_EVENT_NAMES {
  DATA_REQUEST = "fim_debug_data_request",
  DATA_RECEIVE = "fim_debug_data_receive",
  DATA_ERROR = "fim_debug_data_error",
  READY = "fim_debug_ready",
  CLEAR_ERROR = "fim_debug_clear_error",
  BACK = "fim_debug_back",
}

export interface FIMAction {
  type: FIM_EVENT_NAMES;
}

const ALL_FIM_EVENT_NAMES: string[] = Object.values(FIM_EVENT_NAMES);

export function isFIMAction(action: unknown): action is FIMAction {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  return ALL_FIM_EVENT_NAMES.includes(action.type);
}

export interface FIMDebugReady extends FIMAction {
  type: FIM_EVENT_NAMES.READY;
}

export function isReadyMessageFromFIMDebug(
  action: unknown,
): action is FIMDebugReady {
  if (!isFIMAction(action)) return false;
  return action.type === FIM_EVENT_NAMES.READY;
}

export interface RequestFIMData extends FIMAction {
  type: FIM_EVENT_NAMES.DATA_REQUEST;
}

export function isRequestFIMData(action: unknown): action is RequestFIMData {
  if (!isFIMAction(action)) return false;
  return action.type === FIM_EVENT_NAMES.DATA_REQUEST;
}

export interface ClearFIMDebugError extends FIMAction {
  type: FIM_EVENT_NAMES.CLEAR_ERROR;
}

export function isClearFIMDebugError(
  action: unknown,
): action is ClearFIMDebugError {
  if (!isFIMAction(action)) return false;
  return action.type === FIM_EVENT_NAMES.CLEAR_ERROR;
}

export interface ReceiveFIMDebugData extends FIMAction {
  type: FIM_EVENT_NAMES.DATA_RECEIVE;
  payload: FimDebugData;
}

export function isReceiveFIMDebugData(
  action: unknown,
): action is ReceiveFIMDebugData {
  if (!isFIMAction(action)) return false;
  return action.type === FIM_EVENT_NAMES.DATA_RECEIVE;
}

export interface ReceiveFIMDebugError extends FIMAction {
  type: FIM_EVENT_NAMES.DATA_ERROR;
  payload: {
    message: string;
  };
}

export function isReceiveFIMDebugError(
  action: unknown,
): action is ReceiveFIMDebugError {
  if (!isFIMAction(action)) return false;
  if (action.type !== FIM_EVENT_NAMES.DATA_ERROR) return false;
  if (!("payload" in action)) return false;
  if (typeof action.payload !== "object") return false;
  if (action.payload === null) return false;
  if (!("message" in action.payload)) return false;
  return typeof action.payload.message === "string";
}

export interface FIMDebugBack extends FIMAction {
  type: FIM_EVENT_NAMES.BACK;
}

export function isBackFromFIMDebug(action: unknown): action is FIMDebugBack {
  if (!isFIMAction(action)) return false;
  return action.type === FIM_EVENT_NAMES.BACK;
}
