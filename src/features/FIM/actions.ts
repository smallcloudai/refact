import { createAction } from "@reduxjs/toolkit";
import { FimDebugData } from "../../services/refact/fim";

export const request = createAction("fim/request");
export const receive = createAction<FimDebugData>("fim/receive");
export const error = createAction<string>("fim/error");
export const ready = createAction("fim/ready");
export const clearError = createAction("fim/clear_error");
// export const back = createAction("fim/back");
export const reset = createAction("fim/reset");
