import type { StatisticData } from ".";

export enum EVENT_NAMES_FROM_STATISTIC {
  BACK_FROM_STATISTIC = "back_from_statistic",
}

export enum EVENT_NAMES_TO_STATISTIC {
  REQUEST_STATISTIC_DATA = "request_statistic_data",
  RECEIVE_STATISTIC_DATA = "receive_statistic_data",
  RECEIVE_STATISTIC_DATA_ERROR = "receive_statistic_data_error",
  SET_LOADING_STATISTIC_DATA = "set_loading_statistic_data",
  SET_STATISTIC_DATA = "set_statistic_data",
}

interface BaseAction {
  type: EVENT_NAMES_FROM_STATISTIC | EVENT_NAMES_TO_STATISTIC;
  payload?: { data?: string; [key: string]: unknown };
}

export interface ActionFromStatistic extends BaseAction {
  type: EVENT_NAMES_FROM_STATISTIC;
}

export function isActionFromStatistic(
  action: unknown,
): action is ActionFromStatistic {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_FROM_STATISTIC,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export interface ActionToStatistic extends BaseAction {
  type: EVENT_NAMES_TO_STATISTIC;
}

export interface RequestDataForStatistic extends ActionToStatistic {
  type: EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA;
}

export interface ReceiveDataForStatistic extends ActionToStatistic {
  type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA;
  payload: { data: string };
}

export interface SetLoadingStatisticData extends ActionToStatistic {
  type: EVENT_NAMES_TO_STATISTIC.SET_LOADING_STATISTIC_DATA;
}

export function isActionToStatistic(
  action: unknown,
): action is ActionToStatistic {
  if (!action) return false;
  if (typeof action !== "object") return false;
  if (!("type" in action)) return false;
  if (typeof action.type !== "string") return false;
  const ALL_EVENT_NAMES: Record<string, string> = {
    ...EVENT_NAMES_TO_STATISTIC,
  };
  return Object.values(ALL_EVENT_NAMES).includes(action.type);
}

export function isRequestDataForStatistic(
  action: unknown,
): action is RequestDataForStatistic {
  if (!isActionToStatistic(action)) return false;
  return action.type === EVENT_NAMES_TO_STATISTIC.REQUEST_STATISTIC_DATA;
}

export function isReceiveDataForStatistic(
  action: unknown,
): action is ReceiveDataForStatistic {
  if (!isActionToStatistic(action)) return false;
  if (action.type !== EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA)
    return false;
  if (!("payload" in action)) return false;
  if (!action.payload) return false;
  if (typeof action.payload !== "object") return false;
  if (!("data" in action.payload)) return false;
  if (typeof action.payload.data !== "string") return false;
  return true;
}

export interface ReceiveDataForStatisticError extends ActionToStatistic {
  type: EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR;
  payload: {
    data: string;
    message: string;
  };
}

export function isReceiveDataForStatisticError(
  action: unknown,
): action is ReceiveDataForStatisticError {
  if (!isActionToStatistic(action)) return false;
  return action.type === EVENT_NAMES_TO_STATISTIC.RECEIVE_STATISTIC_DATA_ERROR;
}

export interface SetStatisticsData extends ActionToStatistic {
  type: EVENT_NAMES_TO_STATISTIC.SET_STATISTIC_DATA;
  payload: StatisticData;
}

export function isSetStatisticData(
  action: unknown,
): action is SetStatisticsData {
  if (!isActionToStatistic(action)) return false;
  return action.type === EVENT_NAMES_TO_STATISTIC.SET_STATISTIC_DATA;
}

export function isSetLoadingStatisticData(
  action: unknown,
): action is SetLoadingStatisticData {
  if (!isActionToStatistic(action)) return false;
  return action.type === EVENT_NAMES_TO_STATISTIC.SET_LOADING_STATISTIC_DATA;
}
