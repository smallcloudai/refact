import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export type OpenFilesState = {
  files: string[];
};

const initialState: OpenFilesState = {
  files: [],
};

export const openFilesSlice = createSlice({
  name: "openFiles",
  initialState,
  reducers: {
    setOpenFiles: (state, action: PayloadAction<string[]>) => {
      state.files = action.payload;
    },
  },
  selectors: {
    selectOpenFiles: (state) => state.files,
  },
});

export const { setOpenFiles } = openFilesSlice.actions;
export const { selectOpenFiles } = openFilesSlice.selectors;
