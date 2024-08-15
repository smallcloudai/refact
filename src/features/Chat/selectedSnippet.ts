import { createReducer, createAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

export type Snippet = {
  language: string;
  code: string;
  path: string;
  basename: string;
};

const initialState: Snippet = {
  language: "",
  code: "",
  path: "",
  basename: "",
};

// TODO: this event will need to be listened for
export const setSelectedSnippet = createAction<Snippet>("selected_snippet/set");

export const selectedSnippetReducer = createReducer(initialState, (builder) => {
  builder.addCase(setSelectedSnippet, (_state, action) => {
    return action.payload;
  });
});

export const selectSelectedSnippet = (state: RootState) =>
  state.selected_snippet;
