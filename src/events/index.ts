import { request, ready, receive, error } from "../features/FIM";

export const fim = {
  request,
  ready,
  receive,
  error,
};

export * from "./chat";
export * from "../services/refact";
export type * from "../services/refact";
export * from "./sidebar";
export * from "./config";
export type * from "./config";

// TODO: Export events for vscode
export * from "./setup";
export type * from "./setup";
