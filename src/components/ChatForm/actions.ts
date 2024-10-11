import { createAction } from "@reduxjs/toolkit";

export const addInputValue = createAction<string>("textarea/add");
export const setInputValue = createAction<string>("textarea/replace");
