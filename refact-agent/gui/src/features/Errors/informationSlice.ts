import { createSlice, type PayloadAction } from "@reduxjs/toolkit";

export type InformationSliceState = { message: string | null };

const initialState: InformationSliceState = { message: null };
export const informationSlice = createSlice({
  name: "information",
  initialState,
  reducers: {
    setInformation: (state, action: PayloadAction<string>) => {
      if (state.message) return state;
      state.message = action.payload;
    },
    clearInformation: (state, _action: PayloadAction) => {
      state.message = null;
    },
  },
  selectors: {
    getInformationMessage: (state) => state.message,
  },
});

export const { setInformation, clearInformation } = informationSlice.actions;
export const { getInformationMessage } = informationSlice.selectors;
