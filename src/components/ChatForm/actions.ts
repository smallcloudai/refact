import { createAction } from "@reduxjs/toolkit";
import { ChatMessage } from "../../services/refact";

export type InputActionPayload = {
  value?: string;
  messages?: ChatMessage[];
  send_immediately: boolean; // auto_submit flag from customization.yaml
};

export const addInputValue = createAction<InputActionPayload>("textarea/add");
export const setInputValue =
  createAction<InputActionPayload>("textarea/replace");
