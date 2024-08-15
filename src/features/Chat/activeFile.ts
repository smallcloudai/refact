import { createReducer, createAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

export type FileInfo = {
  name: string;
  line1: number | null;
  line2: number | null;
  can_paste: boolean;
  // attach: boolean;
  path: string;
  content?: string;
  usefulness?: number;
  cursor: number | null;
};

const initialState: FileInfo = {
  name: "",
  line1: null,
  line2: null,
  // attach: false,
  can_paste: false,
  path: "",
  cursor: null,
};
// TODO: this event will need to be listened for
export const setFileInfo = createAction<FileInfo>("activeFile/setFileInfo");

export const activeFileReducer = createReducer(initialState, (builder) => {
  builder.addCase(setFileInfo, (state, action) => {
    return { ...state, ...action.payload };
  });
});

export const selectActiveFile = (state: RootState) => state.active_file;
