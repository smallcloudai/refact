import { createReducer } from "@reduxjs/toolkit";
import { request, receive, error, clearError, reset } from "./actions";
import { FimDebugData } from "../../services/refact/fim";
import { RootState } from "../../app/store";

export type FIMDebugState = {
  data: FimDebugData | null;
  error: string | null;
  fetching: boolean;
};

export const initialState: FIMDebugState = {
  data: null,
  error: null,
  fetching: false,
};

export const reducer = createReducer(initialState, (builder) => {
  builder.addCase(request, (state) => {
    state.fetching = true;
  });
  builder.addCase(receive, (state, action) => {
    state.fetching = false;
    state.error = null;
    state.data = action.payload;
  });
  builder.addCase(error, (state, action) => {
    state.fetching = false;
    state.error = action.payload;
  });
  builder.addCase(clearError, (state) => {
    state.error = null;
  });
  builder.addCase(reset, (state) => {
    state.data = null;
    state.error = null;
    state.fetching = false;
  });
});

export const selectFIM = (state: RootState) => state.fim;
