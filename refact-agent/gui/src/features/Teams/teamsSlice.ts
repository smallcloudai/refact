import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { type Workspace } from "../../services/smallcloud/types";

// TODO: shouldn't be unknown

export type TeamsGroup = {
  id: number;
  name: string;
};

type TeamsSliceState = {
  workspace: Workspace | null;
  group: TeamsGroup | null;
};

const initialState: TeamsSliceState = {
  workspace: null,
  group: null,
};

export const teamsSlice = createSlice({
  name: "teams",
  initialState: initialState,
  reducers: {
    setActiveWorkspace: (state, action: PayloadAction<Workspace>) => {
      state.workspace = action.payload;
    },
    resetActiveWorkspace: (state) => {
      state.workspace = null;
    },
    setActiveGroup: (state, action: PayloadAction<TeamsGroup>) => {
      state.group = action.payload;
    },
    resetActiveGroup: (state) => {
      state.group = null;
    },
  },
  selectors: {
    selectActiveWorkspace: (state) => state.workspace,
    selectActiveGroup: (state) => state.group,
  },
});

export const { selectActiveWorkspace, selectActiveGroup } =
  teamsSlice.selectors;
export const {
  setActiveWorkspace,
  resetActiveWorkspace,
  setActiveGroup,
  resetActiveGroup,
} = teamsSlice.actions;
