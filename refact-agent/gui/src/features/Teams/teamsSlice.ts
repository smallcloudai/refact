import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { type TeamsGroup } from "../../services/smallcloud/types";

type TeamsSliceState = {
  group: TeamsGroup | null;
};

const initialState: TeamsSliceState = {
  group: null,
};

export const teamsSlice = createSlice({
  name: "teams",
  initialState: initialState,
  reducers: {
    setActiveGroup: (state, action: PayloadAction<TeamsGroup>) => {
      state.group = action.payload;
    },
    resetActiveGroup: (state) => {
      state.group = null;
    },
  },
  selectors: {
    selectActiveGroup: (state) => state.group,
  },
});

export const { selectActiveGroup } = teamsSlice.selectors;
export const { setActiveGroup, resetActiveGroup } = teamsSlice.actions;
