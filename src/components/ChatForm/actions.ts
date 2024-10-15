import { createAction } from "@reduxjs/toolkit";

export type InputActionPayload = {
  value: string;
  send_immediately: boolean; // auto_submit flag from customization.yaml
};

export const addInputValue = createAction<InputActionPayload>("textarea/add");
export const setInputValue =
  createAction<InputActionPayload>("textarea/replace");
