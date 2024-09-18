import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type WarningSliceState = { message: string[] | null };

const initialState: WarningSliceState = { message: null };
export const warningSlice = createSlice({
  name: "warning",
  initialState,
  reducers: {
    setWarning: (state, action: PayloadAction<string[]>) => {
      if (state.message) return state;
      state.message = action.payload;
    },
    clearWarning: (state, _action: PayloadAction) => {
      state.message = null;
    },
  },
  selectors: {
    getWarningMessage: (state) => state.message,
  },
});

export const { setWarning, clearWarning } = warningSlice.actions;
export const { getWarningMessage } = warningSlice.selectors;
